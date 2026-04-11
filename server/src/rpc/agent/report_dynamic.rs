use crate::entity::dynamic_monitoring;
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::monitoring::data_structure::DynamicMonitoringData;
use nodeget_lib::permission::data_structure::{DynamicMonitoring, Permission, Scope};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use sea_orm::{ActiveValue, EntityTrait, Set};
use serde_json::value::RawValue;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

// 生成唯一错误ID用于内部追踪
static ERROR_COUNTER: AtomicU64 = AtomicU64::new(0);
fn generate_error_id() -> u64 {
    ERROR_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub async fn report_dynamic(
    token: String,
    dynamic_monitoring_data: DynamicMonitoringData,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        let agent_uuid = uuid::Uuid::from_str(&dynamic_monitoring_data.uuid)
            .map_err(|e| NodegetError::ParseError(format!("Invalid UUID format: {e}")))?;

        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(agent_uuid)],
            vec![Permission::DynamicMonitoring(DynamicMonitoring::Write)],
        )
        .await?;

        if !is_allowed {
            return Err(NodegetError::PermissionDenied(
                "Permission Denied: Missing DynamicMonitoring Write permission for this Agent"
                    .to_owned(),
            )
            .into());
        }

        let db = AgentRpcImpl::get_db()?;

        let in_data = dynamic_monitoring::ActiveModel {
            id: ActiveValue::default(),
            uuid: Set(agent_uuid),
            timestamp: Set(dynamic_monitoring_data.time.cast_signed()),
            cpu_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.cpu)
                .map_err(|e| NodegetError::SerializationError(format!("{e}")))?,
            ram_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.ram)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            load_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.load)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            system_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.system)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            disk_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.disk)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            network_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.network)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
            gpu_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.gpu)
                .map_err(|e| NodegetError::SerializationError(e.to_string()))?,
        };

        debug!(target: "rpc", agent_uuid = %dynamic_monitoring_data.uuid, "Received dynamic data");

        let result = dynamic_monitoring::Entity::insert(in_data)
            .exec(db)
            .await
            .map_err(|e| {
                // 内部记录详细错误，向客户端返回通用错误
                let error_id = generate_error_id();
                tracing::error!(target: "rpc", error_id = error_id, error = %e, "Database insert error");
                NodegetError::DatabaseError(format!(
                    "Database error occurred. Reference: {error_id}"
                ))
            })?;

        debug!(target: "rpc", id = result.last_insert_id, "Inserted dynamic data");

        let json_str = format!("{{\"id\":{}}}", result.last_insert_id);
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
