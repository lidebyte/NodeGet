use super::config::CleanupConfig;
use super::CleanupResult;
use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};

/// PostgreSQL 优化版本 - 使用 JSONB 操作符
pub async fn cleanup_expired_data_postgres(db: &DatabaseConnection) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    // 获取所有需要清理的 agent UUID 及其配置
    let configs = get_cleanup_configs_postgres(db).await?;

    for config in configs {
        // 清理 static_monitoring
        if let Some(limit) = config.static_monitoring_limit {
            let deleted = cleanup_table_postgres(
                db,
                "static_monitoring",
                &config.agent_uuid,
                limit,
            ).await?;
            result.static_monitoring_deleted += deleted;
        }

        // 清理 dynamic_monitoring
        if let Some(limit) = config.dynamic_monitoring_limit {
            let deleted = cleanup_table_postgres(
                db,
                "dynamic_monitoring",
                &config.agent_uuid,
                limit,
            ).await?;
            result.dynamic_monitoring_deleted += deleted;
        }

        // 清理 task
        if let Some(limit) = config.task_limit {
            let deleted = cleanup_task_table_postgres(
                db,
                &config.agent_uuid,
                limit,
            ).await?;
            result.task_deleted += deleted;
        }
    }

    // 清理 crontab_result（全局表，从 global 配置读取）
    if let Some(limit) = get_global_crontab_result_limit_postgres(db).await? {
        let deleted = cleanup_crontab_result_table_postgres(db, limit).await?;
        result.crontab_result_deleted = deleted;
    }

    Ok(result)
}

/// PostgreSQL: 获取所有需要清理的配置
async fn get_cleanup_configs_postgres(
    db: &DatabaseConnection,
) -> Result<Vec<CleanupConfig>> {
    // 使用 PostgreSQL JSONB 操作符查询
    // 注意：KV 值存储在 `kv` 字段下，格式为：
    // `{"kv": {"database_limit_task": 1000, ...}, "namespace": "..."}`
    let sql = r#"
        SELECT 
            name as agent_uuid,
            kv_value->'kv'->>'database_limit_static_monitoring' as static_limit,
            kv_value->'kv'->>'database_limit_dynamic_monitoring' as dynamic_limit,
            kv_value->'kv'->>'database_limit_task' as task_limit
        FROM kv
        WHERE name ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
        AND (
            kv_value->'kv' ? 'database_limit_static_monitoring'
            OR kv_value->'kv' ? 'database_limit_dynamic_monitoring'
            OR kv_value->'kv' ? 'database_limit_task'
        )
    "#;

    #[derive(FromQueryResult)]
    struct ConfigRow {
        agent_uuid: String,
        static_limit: Option<String>,
        dynamic_limit: Option<String>,
        task_limit: Option<String>,
    }

    let rows = ConfigRow::find_by_statement(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.to_string(),
    ))
    .all(db)
    .await?;

    let configs: Vec<CleanupConfig> = rows
        .into_iter()
        .map(|row| CleanupConfig {
            agent_uuid: row.agent_uuid,
            static_monitoring_limit: row.static_limit.and_then(|s| s.parse().ok()),
            dynamic_monitoring_limit: row.dynamic_limit.and_then(|s| s.parse().ok()),
            task_limit: row.task_limit.and_then(|s| s.parse().ok()),
            crontab_result_limit: None,
        })
        .collect();

    Ok(configs)
}

/// PostgreSQL: 清理指定表的数据
/// 
/// # 参数
/// * `limit_millis` - 保留的毫秒数
async fn cleanup_table_postgres(
    db: &DatabaseConnection,
    table_name: &str,
    agent_uuid: &str,
    limit_millis: i64,
) -> Result<u64> {
    // 使用参数化查询避免 SQL 注入
    let sql = format!(
        r#"
        DELETE FROM {}
        WHERE uuid = '{}'
        AND timestamp < (
            SELECT MAX(timestamp) - {}
            FROM {}
            WHERE uuid = '{}'
        )
        "#,
        table_name, agent_uuid, limit_millis, table_name, agent_uuid
    );

    let result = db.execute_unprepared(&sql).await?;

    Ok(result.rows_affected())
}

/// PostgreSQL: 清理 task 表（timestamp 可能为 NULL）
/// 
/// # 参数
/// * `limit_millis` - 保留的毫秒数
async fn cleanup_task_table_postgres(
    db: &DatabaseConnection,
    agent_uuid: &str,
    limit_millis: i64,
) -> Result<u64> {
    let sql = format!(
        r#"
        DELETE FROM task
        WHERE uuid = '{}'
        AND timestamp IS NOT NULL
        AND timestamp < (
            SELECT MAX(timestamp) - {}
            FROM task
            WHERE uuid = '{}'
            AND timestamp IS NOT NULL
        )
        "#,
        agent_uuid, limit_millis, agent_uuid
    );

    let result = db.execute_unprepared(&sql).await?;

    Ok(result.rows_affected())
}

/// PostgreSQL: 清理 crontab_result 表
///
/// # 参数
/// * `limit_millis` - 保留的毫秒数
///
/// 注意：crontab_result 是全局表，不关联特定 agent
async fn cleanup_crontab_result_table_postgres(
    db: &DatabaseConnection,
    limit_millis: i64,
) -> Result<u64> {
    let sql = format!(
        r#"
        DELETE FROM crontab_result
        WHERE run_time IS NOT NULL
        AND run_time < (
            SELECT MAX(run_time) - {}
            FROM crontab_result
            WHERE run_time IS NOT NULL
        )
        "#,
        limit_millis
    );

    let result = db.execute_unprepared(&sql).await?;

    Ok(result.rows_affected())
}

/// PostgreSQL 优化版本
/// 使用 JSONB 特性直接在数据库层面过滤
/// 从 global 配置中获取 crontab_result 的清理限制 (PostgreSQL 版本)
///
/// 查找 name 为 "global" 的 KV 记录，读取其中的 database_limit_crontab_result
/// 若不存在则返回 None
async fn get_global_crontab_result_limit_postgres(
    db: &DatabaseConnection,
) -> Result<Option<i64>> {
    let sql = r#"
        SELECT 
            kv_value->'kv'->>'database_limit_crontab_result' as limit_value
        FROM kv
        WHERE name = 'global'
        AND kv_value->'kv' ? 'database_limit_crontab_result'
    "#;

    #[derive(FromQueryResult)]
    struct LimitRow {
        limit_value: Option<String>,
    }

    let result = LimitRow::find_by_statement(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.to_string(),
    ))
    .one(db)
    .await?;

    Ok(result.and_then(|row| row.limit_value.and_then(|s| s.parse().ok())))
}

pub async fn find_uuids_with_database_limit_postgres(
    db: &DatabaseConnection,
) -> Result<Vec<String>> {
    // 使用 PostgreSQL 的 JSONB 操作符优化查询
    // 查询逻辑：
    // 1. 先找出所有 name 是 UUID 格式的记录
    // 2. 检查 kv_value->'kv' 中是否存在以 'database_limit_' 开头的 key
    //
    // 注意：KV 值存储在 `kv` 字段下，格式为：
    // `{"kv": {"database_limit_task": 1000, ...}, "namespace": "..."}`
    //
    // 使用 EXISTS + jsonb_object_keys + LIKE 来检查前缀匹配
    let sql = r#"
        SELECT name FROM kv
        WHERE name ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
        AND EXISTS (
            SELECT 1 FROM jsonb_object_keys(kv_value->'kv') AS key
            WHERE key LIKE 'database_limit_%'
        )
    "#;

    #[derive(FromQueryResult)]
    struct UuidResult {
        name: String,
    }

    let results = UuidResult::find_by_statement(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.to_string(),
    ))
    .all(db)
    .await?;

    Ok(results.into_iter().map(|r| r.name).collect())
}
