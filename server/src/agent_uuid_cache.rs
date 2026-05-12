//! 全局 Agent UUID 缓存。
//!
//! 语义：缓存 **当前在数据库里至少有一条数据的所有 Agent UUID**。
//! 数据来源 = `monitoring_uuid` 表中被 `static_monitoring` / `dynamic_monitoring` /
//! `dynamic_monitoring_summary` 引用的 UUID ∪ `task` 表中的所有 UUID。
//!
//! 生命周期：
//! - [`AgentUuidCache::init`] 在服务启动时一次性从数据库加载全部活跃 UUID 到内存。
//! - [`AgentUuidCache::notify_seen`] 在任何 Agent 上报或新建 Task 时调用：若 UUID 不在缓存则加入。
//!   这是无锁读快路径 + 写锁慢路径的形态，命中率极高情况下几乎零开销。
//! - [`AgentUuidCache::resync`] 在删除任意 monitoring/task 数据后调用：重新查库重建缓存，
//!   剔除已无任何数据残留的 UUID。删除是写操作，本来就要走 DB，再加一次 COUNT/EXISTS
//!   查询开销可接受。
//! - [`AgentUuidCache::list_all`] 用于前端 API 直接读缓存，不走 DB。

use crate::DB;
use nodeget_lib::error::NodegetError;
use sea_orm::{FromQueryResult, Statement};
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(FromQueryResult)]
struct UuidRow {
    uuid: Uuid,
}

/// 内部状态，由 [`AgentUuidCache`] 通过 `RwLock` 保护。
struct AgentUuidCacheInner {
    uuids: HashSet<Uuid>,
}

/// 全局 Agent UUID 缓存。通过 [`AgentUuidCache::global`] 获取单例。
pub struct AgentUuidCache {
    inner: RwLock<AgentUuidCacheInner>,
}

static CACHE: OnceLock<AgentUuidCache> = OnceLock::new();

impl AgentUuidCache {
    /// 首次初始化全局缓存，必须在 `DB` 初始化之后、所有 RPC 路径开放之前调用。
    pub async fn init() -> anyhow::Result<()> {
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;
        let uuids = fetch_active_agent_uuids(db).await?;

        let count = uuids.len();
        let cache = Self {
            inner: RwLock::new(AgentUuidCacheInner { uuids }),
        };

        if CACHE.set(cache).is_err() {
            warn!(target: "agent_uuid", "AgentUuidCache already initialized, reloading");
            Self::resync().await?;
        } else {
            info!(target: "agent_uuid", count, "AgentUuidCache initialized");
        }
        Ok(())
    }

    /// 获取全局缓存实例。若未调用 [`AgentUuidCache::init`] 会 panic。
    pub fn global() -> &'static Self {
        CACHE
            .get()
            .expect("AgentUuidCache not initialized — call AgentUuidCache::init() first")
    }

    /// 通知缓存：某个 UUID 已被 Agent 使用（上报或创建 task）。
    /// 若已在缓存则立即返回；否则加入缓存。幂等。
    pub async fn notify_seen(&self, uuid: Uuid) {
        // 读锁快路径：常见情况下 uuid 已存在
        {
            let guard = self.inner.read().await;
            if guard.uuids.contains(&uuid) {
                return;
            }
        }
        // 写锁慢路径：再次检查 + 插入
        let mut guard = self.inner.write().await;
        if guard.uuids.insert(uuid) {
            debug!(target: "agent_uuid", %uuid, total = guard.uuids.len(), "new agent UUID cached");
        }
    }

    /// 从数据库重建缓存。删除操作后调用，用于剔除所有数据已被删光的 UUID。
    /// 失败时缓存保持不变，并返回错误。
    pub async fn resync() -> anyhow::Result<()> {
        let Some(cache) = CACHE.get() else {
            return Ok(());
        };
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;
        let uuids = fetch_active_agent_uuids(db).await?;
        let count = uuids.len();

        let mut guard = cache.inner.write().await;
        guard.uuids = uuids;
        drop(guard);

        debug!(target: "agent_uuid", count, "AgentUuidCache resynced");
        Ok(())
    }

    /// 返回当前缓存中的全部 UUID，按字典序（字符串）排序，保证稳定输出。
    pub async fn list_all(&self) -> Vec<Uuid> {
        let guard = self.inner.read().await;
        let mut result: Vec<Uuid> = guard.uuids.iter().copied().collect();
        drop(guard); // 提前释放读锁，排序无需持有锁
        result.sort_unstable();
        result
    }

    /// 返回缓存中 UUID 的总数，主要给日志/测试用。
    #[cfg(test)]
    pub async fn len(&self) -> usize {
        self.inner.read().await.uuids.len()
    }
}

/// 从数据库读取所有"活跃" UUID：
/// - `monitoring_uuid` 表中被三张监控表之一引用的 UUID
/// - `task` 表中出现的所有 UUID
/// 两者取并集。
async fn fetch_active_agent_uuids(
    db: &sea_orm::DatabaseConnection,
) -> anyhow::Result<HashSet<Uuid>> {
    let db_backend = db.get_database_backend();

    // 用 EXISTS 索引查询而不是 UNION 全表扫，在有 (uuid_id, timestamp) 复合索引时命中极快
    let sql = r"
        SELECT uuid FROM monitoring_uuid WHERE
          EXISTS (SELECT 1 FROM static_monitoring WHERE uuid_id = monitoring_uuid.id LIMIT 1) OR
          EXISTS (SELECT 1 FROM dynamic_monitoring WHERE uuid_id = monitoring_uuid.id LIMIT 1) OR
          EXISTS (SELECT 1 FROM dynamic_monitoring_summary WHERE uuid_id = monitoring_uuid.id LIMIT 1)
    ";
    let monitoring_rows =
        UuidRow::find_by_statement(Statement::from_string(db_backend, sql.to_owned()))
            .all(db)
            .await
            .map_err(|e| {
                NodegetError::DatabaseError(format!("Failed to query monitoring UUIDs: {e}"))
            })?;

    let task_sql = "SELECT DISTINCT uuid FROM task";
    let task_rows =
        UuidRow::find_by_statement(Statement::from_string(db_backend, task_sql.to_owned()))
            .all(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Failed to query task UUIDs: {e}")))?;

    let mut set: HashSet<Uuid> = HashSet::with_capacity(monitoring_rows.len() + task_rows.len());
    for row in monitoring_rows {
        set.insert(row.uuid);
    }
    for row in task_rows {
        set.insert(row.uuid);
    }
    Ok(set)
}
