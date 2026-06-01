use ng_core::error::NodegetError;
use ng_core::permission::data_structure::Limit;
use ng_core::permission::token_auth::TokenOrAuth;
use ng_db::entity::token;
use ng_infra::make_global_cache;
use ng_infra::server::{DbBackedCache, load_from_db};
use subtle::ConstantTimeEq;
use tracing::{debug, warn};

use crate::get::parse_token_limit_with_compat;
use crate::hash_to_bytes;

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};

const AUTH_FAILED_MESSAGE: &str = "Invalid credentials";

pub struct CachedToken {
    pub model: Arc<token::Model>,
    pub parsed_limits: Vec<Limit>,
    pub token_hash_bytes: [u8; 32],
    pub password_hash_bytes: Option<[u8; 32]>,
}

struct TokenCacheInner {
    by_key: HashMap<String, Arc<CachedToken>>,
    by_username: HashMap<String, Arc<CachedToken>>,
    super_token: Option<Arc<CachedToken>>,
}

pub struct TokenCache {
    inner: RwLock<TokenCacheInner>,
}

fn recover_read(lock: &RwLock<TokenCacheInner>) -> std::sync::RwLockReadGuard<'_, TokenCacheInner> {
    lock.read().unwrap_or_else(|e| {
        tracing::warn!(target: "token_cache", "lock poisoned during read, recovering");
        e.into_inner()
    })
}

fn recover_write(
    lock: &RwLock<TokenCacheInner>,
) -> std::sync::RwLockWriteGuard<'_, TokenCacheInner> {
    lock.write().unwrap_or_else(|e| {
        tracing::warn!(target: "token_cache", "lock poisoned during write, recovering");
        e.into_inner()
    })
}

make_global_cache!(TokenCache, TOKEN_CACHE_GLOBAL);

impl DbBackedCache for TokenCache {
    type Model = token::Model;

    fn cache_name() -> &'static str {
        "token"
    }

    fn build_cache(models: Vec<Self::Model>) -> Self {
        let (by_key, by_username, super_token) = Self::build_maps(models);
        Self {
            inner: RwLock::new(TokenCacheInner {
                by_key,
                by_username,
                super_token,
            }),
        }
    }

    #[allow(clippy::unused_async)]
    async fn reload_from_models(&self, models: Vec<Self::Model>) {
        let (by_key, by_username, super_token) = Self::build_maps(models);
        let mut guard = recover_write(&self.inner);
        guard.by_key = by_key;
        guard.by_username = by_username;
        guard.super_token = super_token;
        drop(guard);
    }

    fn load_all() -> impl Future<Output = anyhow::Result<Vec<Self::Model>>> + Send {
        load_from_db::<token::Entity>()
    }
}

impl TokenCache {
    fn build_maps(
        all_tokens: Vec<token::Model>,
    ) -> (
        HashMap<String, Arc<CachedToken>>,
        HashMap<String, Arc<CachedToken>>,
        Option<Arc<CachedToken>>,
    ) {
        let mut by_key = HashMap::with_capacity(all_tokens.len());
        let mut by_username = HashMap::new();
        let mut super_token: Option<Arc<CachedToken>> = None;

        for model in all_tokens {
            let parsed_limits = parse_token_limit_with_compat(model.token_limit.clone())
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        target: "token",
                        token_key = %model.token_key,
                        error = %e,
                        "failed to pre-parse token_limit, using empty"
                    );
                    Vec::new()
                });

            let token_hash_bytes = hex_to_bytes(&model.token_hash).unwrap_or([0u8; 32]);
            let password_hash_bytes = model.password_hash.as_deref().and_then(hex_to_bytes);

            let cached = Arc::new(CachedToken {
                model: Arc::new(model),
                parsed_limits,
                token_hash_bytes,
                password_hash_bytes,
            });

            if cached.model.id == 1 {
                super_token = Some(Arc::clone(&cached));
            }
            by_key.insert(cached.model.token_key.clone(), Arc::clone(&cached));
            if let Some(ref uname) = cached.model.username {
                by_username.insert(uname.clone(), cached);
            }
        }

        (by_key, by_username, super_token)
    }

    pub fn find_by_key(&self, key: &str) -> Option<Arc<CachedToken>> {
        recover_read(&self.inner).by_key.get(key).map(Arc::clone)
    }

    pub fn find_by_username(&self, username: &str) -> Option<Arc<CachedToken>> {
        recover_read(&self.inner)
            .by_username
            .get(username)
            .map(Arc::clone)
    }

    pub fn get_super_token(&self) -> Option<Arc<CachedToken>> {
        recover_read(&self.inner)
            .super_token
            .as_ref()
            .map(Arc::clone)
    }

    pub fn get_all(&self) -> Vec<Arc<CachedToken>> {
        recover_read(&self.inner)
            .by_key
            .values()
            .map(Arc::clone)
            .collect()
    }

    /// Authenticate in a single lock acquisition.
    ///
    /// Checks the super token entry first, then falls back to regular token lookup.
    /// Returns `(CachedToken, is_super_token)` on success.
    /// Errors if authentication fails or if the super token entry is missing from cache.
    pub fn authenticate(&self, token_or_auth: &TokenOrAuth) -> anyhow::Result<(Arc<CachedToken>, bool)> {
        let inner = recover_read(&self.inner);

        // Ensure super token exists (matches existing check_super_token behavior)
        let super_entry = inner.super_token.as_ref().ok_or_else(|| {
            NodegetError::NotFound("Super Token record (ID 1) not found in cache".to_owned())
        })?;

        match token_or_auth {
            TokenOrAuth::Token(key, secret) => {
                // Check super token first
                let key_match: bool = key
                    .as_bytes()
                    .ct_eq(super_entry.model.token_key.as_bytes())
                    .into();
                if key_match {
                    let computed = hash_to_bytes(secret);
                    let hash_match: bool = computed.ct_eq(&super_entry.token_hash_bytes).into();
                    debug!(target: "auth", is_super = hash_match, "super token check (token auth)");
                    if hash_match {
                        return Ok((Arc::clone(super_entry), true));
                    }
                    // Key matched super token but secret didn't — fall through to regular check
                }

                // Check regular token by key
                if let Some(cached) = inner.by_key.get(key) {
                    let computed = hash_to_bytes(secret);
                    if bool::from(computed.ct_eq(&cached.token_hash_bytes)) {
                        debug!(target: "auth", token_key = %key, "token secret verified successfully");
                        return Ok((Arc::clone(cached), false));
                    }
                    warn!(target: "auth", token_key = %key, "auth failed: invalid token secret");
                    return Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into());
                }

                warn!(target: "auth", token_key = %key, "auth failed: token key not found");
                Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into())
            }
            TokenOrAuth::Auth(username, password) => {
                // Check super token first
                let username_match = super_entry
                    .model
                    .username
                    .as_deref()
                    .is_some_and(|u| u.as_bytes().ct_eq(username.as_bytes()).into());
                if username_match {
                    if let Some(stored) = &super_entry.password_hash_bytes {
                        let computed = hash_to_bytes(password);
                        if bool::from(computed.ct_eq(stored)) {
                            debug!(target: "auth", is_super = true, "authenticate: super token (basic auth)");
                            return Ok((Arc::clone(super_entry), true));
                        }
                        debug!(target: "auth", is_super = false, "super token check (basic auth), password mismatch");
                    }
                    // Username matched super but password didn't (or no password set) — fall through
                }

                // Check regular token by username
                if let Some(cached) = inner.by_username.get(username) {
                    let computed = hash_to_bytes(password);
                    let Some(stored) = &cached.password_hash_bytes else {
                        warn!(target: "auth", username = %username, "auth failed: no password set for this user");
                        return Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into());
                    };
                    if bool::from(computed.ct_eq(stored)) {
                        debug!(target: "auth", username = %username, "password verified successfully");
                        return Ok((Arc::clone(cached), false));
                    }
                    warn!(target: "auth", username = %username, "auth failed: invalid password");
                    return Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into());
                }

                warn!(target: "auth", username = %username, "auth failed: username not found");
                Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into())
            }
        }
    }
}

fn hex_to_bytes(hex_str: &str) -> Option<[u8; 32]> {
    if hex_str.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let hi = hex_str.as_bytes().get(i * 2)?;
        let lo = hex_str.as_bytes().get(i * 2 + 1)?;
        bytes[i] = (hex_nibble(*hi)? << 4) | hex_nibble(*lo)?;
    }
    Some(bytes)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
