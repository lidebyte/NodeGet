use crate::cache::CrontabCache;
use crate::CronType;
use jsonrpsee::core::RpcResult;
use ng_core::error::{NodegetError, anyhow_to_nodeget_error};
use ng_core::permission::data_structure::{Crontab as CrontabPermission, Permission, Scope, Token};
use ng_core::permission::token_auth::TokenOrAuth;
use ng_core::utils::get_local_timestamp_ms_i64;
use ng_token::{check_super_token, get_token};
use serde_json::value::RawValue;
use std::collections::HashSet;
use tracing::{debug, warn};
use uuid::Uuid;

pub async fn get(token: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "crontab", "processing crontab get request");
        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let is_super_token = check_super_token(&token_or_auth)
            .await
            .map_err(|e| NodegetError::PermissionDenied(format!("{e}")))?;

        let cache = CrontabCache::global();
        let entries = cache.get_all_entries();

        if is_super_token {
            let crontabs: Vec<crate::Cron> = entries
                .into_iter()
                .map(|entry| crate::Cron {
                    id: entry.model.id,
                    name: entry.model.name.clone(),
                    enable: entry.model.enable,
                    cron_expression: entry.model.cron_expression.clone(),
                    cron_type: entry.cron_type.clone(),
                    last_run_time: cache.get_last_run_time(entry.model.id, entry.model.last_run_time),
                })
                .collect();

            let json_str = serde_json::to_string(&crontabs).map_err(|e| {
                NodegetError::SerializationError(format!("Failed to serialize crontabs: {e}"))
            })?;

            return RawValue::from_string(json_str)
                .map_err(|e| NodegetError::SerializationError(e.to_string()).into());
        }

        let token_info = get_token(&token_or_auth).await?;

        let now = get_local_timestamp_ms_i64()?;

        if let Some(from) = token_info.timestamp_from
            && now < from
        {
            return Err(NodegetError::PermissionDenied("Token is not yet valid".to_owned()).into());
        }

        if let Some(to) = token_info.timestamp_to
            && now > to
        {
            return Err(NodegetError::PermissionDenied("Token has expired".to_owned()).into());
        }

        let has_crontab_read_permission = token_info.token_limit.iter().any(|limit| {
            limit
                .permissions
                .iter()
                .any(|perm| matches!(perm, Permission::Crontab(CrontabPermission::Read)))
        });

        if !has_crontab_read_permission {
            warn!(target: "crontab", "crontab read permission denied");
            return Err(NodegetError::PermissionDenied(
                "Permission Denied: Insufficient Crontab Read permission".to_owned(),
            )
            .into());
        }

        let crontabs = filter_entries_by_token(&entries, &token_info, cache);
        let json_str = serde_json::to_string(&crontabs).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize crontabs: {e}"))
        })?;

        RawValue::from_string(json_str)
            .map_err(|e| NodegetError::SerializationError(e.to_string()).into())
    };

    match process_logic.await {
        Ok(result) => Ok(result),
        Err(e) => {
            let nodeget_err = anyhow_to_nodeget_error(&e);
            Err(jsonrpsee::types::ErrorObject::owned(
                nodeget_err.error_code() as i32,
                format!("{nodeget_err}"),
                None::<()>,
            ))
        }
    }
}

fn filter_entries_by_token(
    entries: &[std::sync::Arc<crate::cache::CachedCrontab>],
    token_info: &Token,
    cache: &CrontabCache,
) -> Vec<crate::Cron> {
    let mut has_global = false;
    let mut allowed_uuids: HashSet<Uuid> = HashSet::new();

    for limit in &token_info.token_limit {
        let has_crontab_read = limit
            .permissions
            .iter()
            .any(|p| matches!(p, Permission::Crontab(CrontabPermission::Read)));

        if !has_crontab_read {
            continue;
        }

        for scope in &limit.scopes {
            match scope {
                Scope::Global => {
                    has_global = true;
                }
                Scope::AgentUuid(uuid) => {
                    allowed_uuids.insert(*uuid);
                }
                Scope::KvNamespace(_)
                | Scope::JsWorker(_)
                | Scope::StaticBucket(_)
                | Scope::Db(_) => {
                    // 不适用于 crontab 权限检查，忽略
                }
            }
        }
    }

    entries
        .iter()
        .filter(|entry| {
            if has_global {
                return true;
            }
            match &entry.cron_type {
                CronType::Agent(agent_uuids, _) => {
                    agent_uuids.iter().any(|uuid| allowed_uuids.contains(uuid))
                }
                CronType::Server(_) => false,
            }
        })
        .map(|entry| crate::Cron {
            id: entry.model.id,
            name: entry.model.name.clone(),
            enable: entry.model.enable,
            cron_expression: entry.model.cron_expression.clone(),
            cron_type: entry.cron_type.clone(),
            last_run_time: cache.get_last_run_time(entry.model.id, entry.model.last_run_time),
        })
        .collect()
}
