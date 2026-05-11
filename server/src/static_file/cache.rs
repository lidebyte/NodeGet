use crate::DB;
use crate::entity::static_file as static_entity;
use nodeget_lib::error::NodegetError;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct CachedStatic {
    pub model: Arc<static_entity::Model>,
}

struct StaticCacheInner {
    by_name: HashMap<String, CachedStatic>,
    http_root_name: Option<String>,
}

pub struct StaticCache {
    inner: RwLock<StaticCacheInner>,
}

static CACHE: OnceLock<StaticCache> = OnceLock::new();

impl StaticCache {
    pub async fn init() -> anyhow::Result<()> {
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;

        let all = static_entity::Entity::find()
            .all(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Failed to load static: {e}")))?;

        let (by_name, http_root_name) = Self::build_cache(all);
        let count = by_name.len();
        let cache = Self {
            inner: RwLock::new(StaticCacheInner {
                by_name,
                http_root_name,
            }),
        };

        if CACHE.set(cache).is_err() {
            warn!(target: "static", "StaticCache already initialized, reloading");
            Self::reload().await?;
        } else {
            info!(target: "static", count, "StaticCache initialized");
        }

        Ok(())
    }

    pub fn global() -> &'static Self {
        CACHE
            .get()
            .expect("StaticCache not initialized — call StaticCache::init() first")
    }

    pub async fn reload() -> anyhow::Result<()> {
        let Some(cache) = CACHE.get() else {
            return Ok(());
        };
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;

        let all = static_entity::Entity::find()
            .all(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Failed to reload static: {e}")))?;

        let (by_name, http_root_name) = Self::build_cache(all);
        let mut guard = cache.inner.write().await;
        guard.by_name = by_name;
        guard.http_root_name = http_root_name;
        drop(guard);

        debug!(target: "static", "StaticCache reloaded");
        Ok(())
    }

    pub async fn get_by_name(&self, name: &str) -> Option<Arc<static_entity::Model>> {
        let guard = self.inner.read().await;
        guard.by_name.get(name).map(|c| Arc::clone(&c.model))
    }

    pub async fn get_http_root(&self) -> Option<Arc<static_entity::Model>> {
        let guard = self.inner.read().await;
        let name = guard.http_root_name.as_ref()?;
        guard.by_name.get(name).map(|c| Arc::clone(&c.model))
    }

    pub async fn get_all(&self) -> Vec<Arc<static_entity::Model>> {
        let guard = self.inner.read().await;
        guard
            .by_name
            .values()
            .map(|c| Arc::clone(&c.model))
            .collect()
    }

    pub async fn exists(&self, name: &str) -> bool {
        let guard = self.inner.read().await;
        guard.by_name.contains_key(name)
    }

    fn build_cache(
        models: Vec<static_entity::Model>,
    ) -> (HashMap<String, CachedStatic>, Option<String>) {
        let mut by_name = HashMap::with_capacity(models.len());
        let mut http_root_name = None;

        for model in models {
            if model.is_http_root {
                if http_root_name.is_none() {
                    http_root_name = Some(model.name.clone());
                } else {
                    warn!(
                        target: "static",
                        name = %model.name,
                        existing = %http_root_name.as_ref().unwrap_or(&"unknown".to_owned()),
                        "duplicate is_http_root detected, ignoring"
                    );
                }
            }
            let name = model.name.clone();
            by_name.insert(
                name,
                CachedStatic {
                    model: Arc::new(model),
                },
            );
        }

        (by_name, http_root_name)
    }
}
