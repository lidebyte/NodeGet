use crate::entity::dynamic_monitoring;
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use log::debug;
use nodeget_lib::monitoring::data_structure::DynamicMonitoringData;
use nodeget_lib::permission::data_structure::{Permission, Scope};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use sea_orm::{ActiveValue, EntityTrait, Set};
use serde_json::{Value, json};
use std::str::FromStr;

pub async fn report_dynamic(
    token: String,
    dynamic_monitoring_data: DynamicMonitoringData,
) -> Value {
    let process_logic = async {
        let agent_uuid = uuid::Uuid::from_str(&dynamic_monitoring_data.uuid)
            .map_err(|e| (101, format!("Invalid UUID format: {e}")))?;

        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => return Err((101, format!("Failed to parse token: {e}"))),
        };

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(agent_uuid)],
            vec![Permission::DynamicMonitoring(
                nodeget_lib::permission::data_structure::DynamicMonitoring::Write,
            )],
        )
        .await?;

        if !is_allowed {
            return Err((
                102,
                "Permission Denied: Missing DynamicMonitoring Write permission for this Agent"
                    .to_string(),
            ));
        }

        let db = AgentRpcImpl::get_db()?;

        let in_data = dynamic_monitoring::ActiveModel {
            id: ActiveValue::default(),
            uuid: Set(agent_uuid),
            timestamp: Set(dynamic_monitoring_data.time.cast_signed()),

            cpu_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.cpu)
                .map_err(|e| (101, e))?,
            ram_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.ram)
                .map_err(|e| (101, e))?,
            load_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.load)
                .map_err(|e| (101, e))?,
            system_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.system)
                .map_err(|e| (101, e))?,
            disk_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.disk)
                .map_err(|e| (101, e))?,
            network_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.network)
                .map_err(|e| (101, e))?,
            gpu_data: AgentRpcImpl::try_set_json(dynamic_monitoring_data.gpu)
                .map_err(|e| (101, e))?,
        };

        debug!(
            "Received dynamic data from [{}]",
            dynamic_monitoring_data.uuid.clone()
        );

        let result = dynamic_monitoring::Entity::insert(in_data)
            .exec(db)
            .await
            .map_err(|e| {
                log::error!("Database insert error: {e}");
                (103, format!("Database insert error: {e}"))
            })?;

        debug!("Inserted dynamic data with id [{}]", result.last_insert_id);

        Ok(result.last_insert_id)
    };

    match process_logic.await {
        Ok(new_id) => json!({ "id": new_id }),
        Err((code, msg)) => generate_error_message(code, &msg),
    }
}
