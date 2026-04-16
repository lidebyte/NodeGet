use crate::entity::static_monitoring;
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::monitoring::data_structure::StaticMonitoringData;
use nodeget_lib::permission::data_structure::{Permission, Scope, StaticMonitoring};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use sea_orm::{ActiveValue, Set};
use serde_json::value::RawValue;
use std::str::FromStr;
use tracing::debug;

pub async fn report_static(
    token: String,
    static_monitoring_data: StaticMonitoringData,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        let agent_uuid = uuid::Uuid::from_str(&static_monitoring_data.uuid)
            .map_err(|e| NodegetError::ParseError(format!("Invalid UUID format: {e}")))?;

        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(agent_uuid)],
            vec![Permission::StaticMonitoring(StaticMonitoring::Write)],
        )
        .await?;

        if !is_allowed {
            return Err(NodegetError::PermissionDenied(
                "Permission Denied: Missing StaticMonitoring Write permission for this Agent"
                    .to_string(),
            )
            .into());
        }

        let in_data = static_monitoring::ActiveModel {
            id: ActiveValue::default(),
            uuid: Set(agent_uuid),
            timestamp: Set(static_monitoring_data.time.cast_signed()),
            cpu_data: AgentRpcImpl::try_set_json(static_monitoring_data.cpu)
                .map_err(|e| NodegetError::SerializationError(format!("{e}")))?,
            system_data: AgentRpcImpl::try_set_json(static_monitoring_data.system)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            gpu_data: AgentRpcImpl::try_set_json(static_monitoring_data.gpu)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
        };

        debug!(target: "monitoring", agent_uuid = %static_monitoring_data.uuid, "Received static data");

        crate::monitoring_buffer::get()
            .static_mon
            .send(in_data)
            .map_err(|_| NodegetError::DatabaseError("Buffer closed".to_owned()))?;

        RawValue::from_string(r#"{"status":"buffered"}"#.to_owned())
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
