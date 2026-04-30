//! In-memory cache for static monitoring `data_hash` deduplication.
//! Avoids a DB query on every `report_static` call when the hash hasn't changed.

use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

struct Inner {
    /// `uuid_id` -> last known `data_hash`
    by_uuid_id: HashMap<i16, Vec<u8>>,
}

pub struct StaticHashCache {
    inner: RwLock<Inner>,
}

static CACHE: OnceLock<StaticHashCache> = OnceLock::new();

impl StaticHashCache {
    /// Initialize the global cache (empty — populated lazily on first report per uuid).
    pub fn init() {
        CACHE.get_or_init(|| Self {
            inner: RwLock::new(Inner {
                by_uuid_id: HashMap::new(),
            }),
        });
    }

    /// Get the global instance.
    pub fn global() -> &'static Self {
        CACHE
            .get()
            .expect("StaticHashCache not initialized — call StaticHashCache::init() first")
    }

    /// Check if the given hash matches the cached hash for this `uuid_id`.
    /// Returns `true` if it's a known duplicate (same hash as last time).
    pub async fn is_duplicate(&self, uuid_id: i16, data_hash: &[u8]) -> bool {
        let guard = self.inner.read().await;
        guard
            .by_uuid_id
            .get(&uuid_id)
            .is_some_and(|cached| cached == data_hash)
    }

    /// Update the cached hash for a `uuid_id` after successful buffer/insert.
    pub async fn update(&self, uuid_id: i16, data_hash: Vec<u8>) {
        let mut guard = self.inner.write().await;
        guard.by_uuid_id.insert(uuid_id, data_hash);
    }
}
