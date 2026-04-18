use crate::crontab::set_crontab_enable_by_name;
use crate::entity::crontab;
use crate::rpc::RpcHelper;
use crate::rpc::crontab::CrontabRpcImpl;
use crate::rpc::crontab::auth::{ensure_crontab_scope_permission, parse_cron_type};
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::{Crontab as CrontabPermission, Permission};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::value::RawValue;
use tracing::debug;

pub async fn set_enable(token: String, name: String, enable: bool) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "crontab", name = %name, enable = enable, "processing crontab set_enable request");
        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let db = CrontabRpcImpl::get_db()?;
        let model = crontab::Entity::find()
            .filter(crontab::Column::Name.eq(&name))
            .one(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(e.to_string()))?;

        let Some(model) = model else {
            return Err(NodegetError::NotFound(format!("Crontab not found: {name}")).into());
        };

        let cron_type = parse_cron_type(&model.cron_type, &name)?;
        ensure_crontab_scope_permission(
            &token_or_auth,
            &cron_type,
            Permission::Crontab(CrontabPermission::Write),
            "Permission Denied: Missing Crontab Write permission for all target scopes",
        )
        .await?;

        let result_state = set_crontab_enable_by_name(name.clone(), enable)
            .await
            .map_err(|e| NodegetError::Other(format!("Failed to set crontab enable: {e}")))?;

        let state = result_state
            .ok_or_else(|| NodegetError::NotFound(format!("Crontab not found: {name}")))?;

        debug!(target: "crontab", name = %name, enabled = state, "Crontab enable state updated");

        let json_str = format!("{{\"success\":true,\"enabled\":{state}}}");

        RawValue::from_string(json_str)
            .map_err(|e| NodegetError::SerializationError(e.to_string()).into())
    };

    match process_logic.await {
        Ok(result) => Ok(result),
        Err(e) => {
            let nodeget_err = nodeget_lib::error::anyhow_to_nodeget_error(&e);
            Err(jsonrpsee::types::ErrorObject::owned(
                nodeget_err.error_code() as i32,
                format!("{nodeget_err}"),
                None::<()>,
            ))
        }
    }
}
