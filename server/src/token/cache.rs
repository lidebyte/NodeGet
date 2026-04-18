use crate::DB;
use crate::entity::token;
use nodeget_lib::error::NodegetError;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

struct TokenCacheInner {
    /// token_key -> Model
    by_key: HashMap<String, token::Model>,
    /// username -> Model (only tokens that have a username)
    by_username: HashMap<String, token::Model>,
    /// super token (id=1), cached separately for fast access
    super_token: Option<token::Model>,
}

pub struct TokenCache {
    inner: RwLock<TokenCacheInner>,
}

static TOKEN_CACHE: OnceLock<TokenCache> = OnceLock::new();

impl TokenCache {
    /// Initialize the global token cache by loading all tokens from DB.
    /// Must be called after DB is initialized and super token is created.
    pub async fn init() -> anyhow::Result<()> {
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;

        let all_tokens = token::Entity::find()
            .all(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Failed to load tokens: {e}")))?;

        let mut by_key = HashMap::with_capacity(all_tokens.len());
        let mut by_username = HashMap::new();
        let mut super_token = None;

        for model in all_tokens {
            if model.id == 1 {
                super_token = Some(model.clone());
            }
            by_key.insert(model.token_key.clone(), model.clone());
            if let Some(ref uname) = model.username {
                by_username.insert(uname.clone(), model);
            }
        }

        let cache = TokenCache {
            inner: RwLock::new(TokenCacheInner {
                by_key,
                by_username,
                super_token,
            }),
        };

        if TOKEN_CACHE.set(cache).is_err() {
            // Already initialized — just reload instead
            tracing::warn!(target: "token", "Token cache already initialized, reloading");
            Self::reload().await?;
        } else {
            tracing::info!(target: "token", "Token cache initialized");
        }

        Ok(())
    }

    /// Check if the global token cache has been initialized.
    pub fn is_initialized() -> bool {
        TOKEN_CACHE.get().is_some()
    }

    /// Get the global token cache instance.
    pub fn global() -> &'static TokenCache {
        TOKEN_CACHE
            .get()
            .expect("Token cache not initialized — call TokenCache::init() first")
    }

    /// Reload all tokens from DB into cache.
    /// Called after any CUD operation on the token table.
    /// No-op if cache hasn't been initialized yet (e.g. during startup).
    pub async fn reload() -> anyhow::Result<()> {
        let Some(cache) = TOKEN_CACHE.get() else {
            // Cache not yet initialized — init() will load everything
            return Ok(());
        };
        let db = DB.get().ok_or_else(|| {
            NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
        })?;

        let all_tokens = token::Entity::find()
            .all(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Failed to reload tokens: {e}")))?;

        let mut by_key = HashMap::with_capacity(all_tokens.len());
        let mut by_username = HashMap::new();
        let mut super_token = None;

        for model in all_tokens {
            if model.id == 1 {
                super_token = Some(model.clone());
            }
            by_key.insert(model.token_key.clone(), model.clone());
            if let Some(ref uname) = model.username {
                by_username.insert(uname.clone(), model);
            }
        }

        let mut guard = cache.inner.write().await;
        guard.by_key = by_key;
        guard.by_username = by_username;
        guard.super_token = super_token;

        tracing::debug!(target: "token", "Token cache reloaded");
        Ok(())
    }

    /// Find a token model by token_key.
    pub async fn find_by_key(&self, key: &str) -> Option<token::Model> {
        let guard = self.inner.read().await;
        guard.by_key.get(key).cloned()
    }

    /// Find a token model by username.
    pub async fn find_by_username(&self, username: &str) -> Option<token::Model> {
        let guard = self.inner.read().await;
        guard.by_username.get(username).cloned()
    }

    /// Get the super token model (id=1).
    pub async fn get_super_token(&self) -> Option<token::Model> {
        let guard = self.inner.read().await;
        guard.super_token.clone()
    }

    /// Get all token models (for list_all_tokens).
    pub async fn get_all(&self) -> Vec<token::Model> {
        let guard = self.inner.read().await;
        guard.by_key.values().cloned().collect()
    }
}
