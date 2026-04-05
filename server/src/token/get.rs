use crate::DB;
use crate::entity::token;
use crate::token::hash_string;
use crate::token::super_token::check_super_token;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::{
    CrontabResult, JsResult, Kv, Limit, Permission, Scope, Token,
};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::get_local_timestamp_ms_i64;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use subtle::ConstantTimeEq;

/// 统一的身份验证失败错误消息，防止信息泄露
const AUTH_FAILED_MESSAGE: &str = "Invalid credentials";

/// 使用恒定时间比较验证哈希，防止时序攻击
fn verify_hash_constant_time(computed_hash: &str, stored_hash: &str) -> bool {
    computed_hash
        .as_bytes()
        .ct_eq(stored_hash.as_bytes())
        .into()
}

pub async fn get_token(token_or_auth: &TokenOrAuth) -> anyhow::Result<Token> {
    let db = DB.get().ok_or_else(|| {
        NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
    })?;

    let token_model = match token_or_auth {
        TokenOrAuth::Token(key, secret) => {
            let model = token::Entity::find()
                .filter(token::Column::TokenKey.eq(key))
                .one(db)
                .await
                .map_err(|e| NodegetError::DatabaseError(format!("Database query error: {e}")))?
                .ok_or_else(|| NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()))?;

            let computed_hash = hash_string(secret);
            if !verify_hash_constant_time(&computed_hash, &model.token_hash) {
                return Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into());
            }

            model
        }
        TokenOrAuth::Auth(username, password) => {
            let model = token::Entity::find()
                .filter(token::Column::Username.eq(username))
                .one(db)
                .await
                .map_err(|e| NodegetError::DatabaseError(format!("Database query error: {e}")))?
                .ok_or_else(|| NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()))?;

            let p_hash = hash_string(password);
            let stored_hash = model.password_hash.as_deref().unwrap_or("");
            if !verify_hash_constant_time(&p_hash, stored_hash) {
                return Err(NodegetError::PermissionDenied(AUTH_FAILED_MESSAGE.to_owned()).into());
            }

            model
        }
    };

    let token_limit = parse_token_limit_with_compat(token_model.token_limit)?;

    Ok(Token {
        version: token_model.version,
        token_key: token_model.token_key,
        timestamp_from: token_model.time_stamp_from,
        timestamp_to: token_model.time_stamp_to,
        token_limit,
        username: token_model.username,
    })
}

pub async fn get_token_by_key_or_username(identifier: &str) -> anyhow::Result<Token> {
    let db = DB.get().ok_or_else(|| {
        NodegetError::ConfigNotFound("Database connection not initialized".to_owned())
    })?;

    let token_model = if let Some(model) = token::Entity::find()
        .filter(token::Column::TokenKey.eq(identifier))
        .one(db)
        .await
        .map_err(|e| NodegetError::DatabaseError(format!("Database query error: {e}")))?
    {
        model
    } else {
        token::Entity::find()
            .filter(token::Column::Username.eq(identifier))
            .one(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(format!("Database query error: {e}")))?
            .ok_or_else(|| {
                NodegetError::NotFound(format!("Token not found by key/username: {identifier}"))
            })?
    };

    let token_limit = parse_token_limit_with_compat(token_model.token_limit)?;

    Ok(Token {
        version: token_model.version,
        token_key: token_model.token_key,
        timestamp_from: token_model.time_stamp_from,
        timestamp_to: token_model.time_stamp_to,
        token_limit,
        username: token_model.username,
    })
}

fn drop_unknown_permissions(mut token_limit_value: Value) -> Value {
    let Some(limits) = token_limit_value.as_array_mut() else {
        return token_limit_value;
    };

    for limit in limits.iter_mut() {
        let Some(perms) = limit.get_mut("permissions").and_then(Value::as_array_mut) else {
            continue;
        };

        perms.retain(|perm| serde_json::from_value::<Permission>(perm.clone()).is_ok());
    }

    token_limit_value
}

pub fn parse_token_limit_with_compat(token_limit_value: Value) -> anyhow::Result<Vec<Limit>> {
    match serde_json::from_value::<Vec<Limit>>(token_limit_value.clone()) {
        Ok(v) => Ok(v),
        Err(original_err) => {
            let filtered = drop_unknown_permissions(token_limit_value);
            serde_json::from_value::<Vec<Limit>>(filtered).map_err(|e| {
                NodegetError::SerializationError(format!(
                    "Failed to parse token permissions: {e}; original error: {original_err}"
                ))
                .into()
            })
        }
    }
}

/// 通配符匹配函数 - 仅支持后缀通配符 `*`
///
/// # 说明
/// - `pattern` 以 `*` 结尾时，匹配以 `*` 前内容开头的任意字符串
/// - `pattern` 不以 `*` 结尾时，进行精确匹配
///
/// # 示例
/// - `wildcard_matches_pattern("abc", "ab*")` -> true
/// - `wildcard_matches_pattern("abc", "abc")` -> true  
/// - `wildcard_matches_pattern("abc", "a*")` -> true
/// - `wildcard_matches_pattern("abc", "xyz")` -> false
fn wildcard_matches_pattern(value: &str, pattern: &str) -> bool {
    pattern
        .strip_suffix('*')
        .map_or_else(|| value == pattern, |prefix| value.starts_with(prefix))
}

fn permission_matches(granted: &Permission, required: &Permission) -> bool {
    if granted == required {
        return true;
    }

    match (granted, required) {
        (Permission::Kv(Kv::Read(pattern)), Permission::Kv(Kv::Read(key)))
        | (Permission::Kv(Kv::Write(pattern)), Permission::Kv(Kv::Write(key)))
        | (Permission::Kv(Kv::Delete(pattern)), Permission::Kv(Kv::Delete(key))) => {
            wildcard_matches_pattern(key, pattern)
        }
        (
            Permission::CrontabResult(CrontabResult::Read(pattern)),
            Permission::CrontabResult(CrontabResult::Read(cron_name)),
        )
        | (
            Permission::CrontabResult(CrontabResult::Delete(pattern)),
            Permission::CrontabResult(CrontabResult::Delete(cron_name)),
        ) => wildcard_matches_pattern(cron_name, pattern),
        (
            Permission::JsResult(JsResult::Read(pattern)),
            Permission::JsResult(JsResult::Read(worker_name)),
        )
        | (
            Permission::JsResult(JsResult::Delete(pattern)),
            Permission::JsResult(JsResult::Delete(worker_name)),
        ) => wildcard_matches_pattern(worker_name, pattern),
        _ => false,
    }
}

fn scope_matches(limit_scope: &Scope, req_scope: &Scope) -> bool {
    match (limit_scope, req_scope) {
        (Scope::Global, _) => true,
        (Scope::AgentUuid(limit_id), Scope::AgentUuid(req_id)) => limit_id == req_id,
        (Scope::KvNamespace(limit_ns), Scope::KvNamespace(req_ns)) => limit_ns == req_ns,
        (Scope::JsWorker(limit_name), Scope::JsWorker(req_name)) => {
            wildcard_matches_pattern(req_name, limit_name)
        }
        (Scope::AgentUuid(_) | Scope::KvNamespace(_) | Scope::JsWorker(_), Scope::Global)
        | (Scope::AgentUuid(_), Scope::KvNamespace(_) | Scope::JsWorker(_))
        | (Scope::KvNamespace(_), Scope::AgentUuid(_) | Scope::JsWorker(_))
        | (Scope::JsWorker(_), Scope::AgentUuid(_) | Scope::KvNamespace(_)) => false,
    }
}

pub async fn check_token_limit(
    token_or_auth: &TokenOrAuth,
    scopes: Vec<Scope>,
    permissions: Vec<Permission>,
) -> anyhow::Result<bool> {
    let is_super_token = check_super_token(token_or_auth)
        .await
        .map_err(|e| NodegetError::PermissionDenied(format!("{e}")))?;
    if is_super_token {
        return Ok(true);
    }

    let token = get_token(token_or_auth).await?;

    let now = get_local_timestamp_ms_i64()?;
    if let Some(from) = token.timestamp_from
        && now < from
    {
        return Ok(false);
    }
    if let Some(to) = token.timestamp_to
        && now > to
    {
        return Ok(false);
    }

    for req_scope in &scopes {
        for req_perm in &permissions {
            let mut is_allowed = false;

            for limit in &token.token_limit {
                let scope_covered = limit
                    .scopes
                    .iter()
                    .any(|limit_scope| scope_matches(limit_scope, req_scope));
                if !scope_covered {
                    continue;
                }

                if limit
                    .permissions
                    .iter()
                    .any(|perm| permission_matches(perm, req_perm))
                {
                    is_allowed = true;
                    break;
                }
            }

            if !is_allowed {
                return Ok(false);
            }
        }
    }

    Ok(true)
}
