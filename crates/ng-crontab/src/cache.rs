use crate::CronType;
use cron::Schedule;
use ng_db::entity::crontab;
use ng_infra::make_global_cache;
use ng_infra::server::{DbBackedCache, load_from_db};
use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tracing::warn;

pub struct CachedCrontab {
    pub model: Arc<crontab::Model>,
    pub schedule: Schedule,
    pub cron_type: CronType,
}

struct CrontabCacheInner {
    by_id: HashMap<i64, Arc<CachedCrontab>>,
    /// Separate tracking of last_run_time to avoid deep-cloning entire CachedCrontab.
    /// Key = crontab id, Value = last_run_time millis. Checked before model.last_run_time.
    last_run_times: RwLock<HashMap<i64, i64>>,
}

pub struct CrontabCache {
    inner: RwLock<CrontabCacheInner>,
}

fn recover_read(
    lock: &RwLock<CrontabCacheInner>,
) -> std::sync::RwLockReadGuard<'_, CrontabCacheInner> {
    lock.read().unwrap_or_else(|e| {
        tracing::warn!(target: "crontab_cache", "lock poisoned during read, recovering");
        e.into_inner()
    })
}

fn recover_write(
    lock: &RwLock<CrontabCacheInner>,
) -> std::sync::RwLockWriteGuard<'_, CrontabCacheInner> {
    lock.write().unwrap_or_else(|e| {
        tracing::warn!(target: "crontab_cache", "lock poisoned during write, recovering");
        e.into_inner()
    })
}

make_global_cache!(CrontabCache, CRONTAB_CACHE_GLOBAL);

impl DbBackedCache for CrontabCache {
    type Model = crontab::Model;

    fn cache_name() -> &'static str {
        "crontab"
    }

    fn build_cache(models: Vec<Self::Model>) -> Self {
        let by_id = Self::build_maps(models);
        Self {
            inner: RwLock::new(CrontabCacheInner {
                by_id,
                last_run_times: RwLock::new(HashMap::new()),
            }),
        }
    }

    #[allow(clippy::unused_async)]
    async fn reload_from_models(&self, models: Vec<Self::Model>) {
        let by_id = Self::build_maps(models);
        let mut guard = recover_write(&self.inner);
        guard.by_id = by_id;
        drop(guard);
    }

    fn load_all() -> impl Future<Output = anyhow::Result<Vec<Self::Model>>> + Send {
        load_from_db::<crontab::Entity>()
    }
}

impl CrontabCache {
    fn build_maps(models: Vec<crontab::Model>) -> HashMap<i64, Arc<CachedCrontab>> {
        let mut by_id = HashMap::with_capacity(models.len());
        for model in models {
            let schedule = match Schedule::from_str(&model.cron_expression) {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        target: "crontab",
                        job_id = model.id,
                        job_name = %model.name,
                        error = %e,
                        "invalid cron expression during cache build, skipping"
                    );
                    continue;
                }
            };

            let cron_type = match serde_json::from_value::<CronType>(model.cron_type.clone()) {
                Ok(ct) => ct,
                Err(e) => {
                    warn!(
                        target: "crontab",
                        job_id = model.id,
                        job_name = %model.name,
                        error = %e,
                        "invalid cron_type during cache build, skipping"
                    );
                    continue;
                }
            };

            let id = model.id;
            by_id.insert(
                id,
                Arc::new(CachedCrontab {
                    model: Arc::new(model),
                    schedule,
                    cron_type,
                }),
            );
        }
        by_id
    }

    pub fn get_enabled_entries(&self) -> Vec<Arc<CachedCrontab>> {
        let guard = recover_read(&self.inner);
        guard
            .by_id
            .values()
            .filter(|entry| entry.model.enable)
            .map(Arc::clone)
            .collect()
    }

    /// Returns all cached crontab entries (including disabled ones).
    /// For use by the `get` RPC handler to avoid hitting DB.
    pub fn get_all_entries(&self) -> Vec<Arc<CachedCrontab>> {
        let guard = recover_read(&self.inner);
        guard.by_id.values().map(Arc::clone).collect()
    }

    /// Get the effective last_run_time for a crontab entry.
    /// Checks the override map first, falls back to model.last_run_time.
    pub fn get_last_run_time(&self, id: i64, model_last: Option<i64>) -> Option<i64> {
        let guard = recover_read(&self.inner);
        guard
            .last_run_times
            .read()
            .unwrap_or_else(|e| {
                warn!(target: "crontab_cache", "last_run_times lock poisoned during read, recovering");
                e.into_inner()
            })
            .get(&id)
            .copied()
            .or(model_last)
    }

    pub fn update_last_run_time(&self, id: i64, timestamp: i64) {
        let guard = recover_read(&self.inner);
        let mut lrt_guard = guard.last_run_times.write().unwrap_or_else(|e| {
            warn!(target: "crontab_cache", "last_run_times lock poisoned during write, recovering");
            e.into_inner()
        });
        lrt_guard.insert(id, timestamp);
    }
}
