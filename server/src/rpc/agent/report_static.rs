use crate::entity::static_monitoring;
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use log::debug;
use nodeget_lib::monitoring::data_structure::StaticMonitoringData;
use nodeget_lib::permission::data_structure::{Permission, Scope, StaticMonitoring};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use sea_orm::{ActiveValue, EntityTrait, Set};
use serde_json::{Value, json};
use std::str::FromStr;

pub async fn report_static(token: String, static_monitoring_data: StaticMonitoringData) -> Value {
    let process_logic = async {
        let agent_uuid = uuid::Uuid::from_str(&static_monitoring_data.uuid)
            .map_err(|e| (101, format!("Invalid UUID format: {e}")))?;

        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => return Err((101, format!("Failed to parse token: {e}"))),
        };

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(agent_uuid)],
            vec![Permission::StaticMonitoring(StaticMonitoring::Write)],
        )
        .await?;

        if !is_allowed {
            return Err((
                102,
                "Permission Denied: Missing StaticMonitoring Write permission for this Agent"
                    .to_string(),
            ));
        }

        let db = AgentRpcImpl::get_db()?;

        let in_data = static_monitoring::ActiveModel {
            id: ActiveValue::default(),
            uuid: Set(agent_uuid),
            timestamp: Set(static_monitoring_data.time.cast_signed()),

            cpu_data: AgentRpcImpl::try_set_json(static_monitoring_data.cpu)
                .map_err(|e| (101, e))?,
            system_data: AgentRpcImpl::try_set_json(static_monitoring_data.system)
                .map_err(|e| (101, e))?,
            gpu_data: AgentRpcImpl::try_set_json(static_monitoring_data.gpu)
                .map_err(|e| (101, e))?,
        };

        debug!(
            "Received static data from [{}]",
            static_monitoring_data.uuid.clone()
        );

        let result = static_monitoring::Entity::insert(in_data)
            .exec(db)
            .await
            .map_err(|e| {
                log::error!("Database insert error: {e}");
                (103, format!("Database insert error: {e}"))
            })?;

        debug!("Inserted static data with id [{}]", result.last_insert_id);

        Ok(result.last_insert_id)
    };

    match process_logic.await {
        Ok(new_id) => json!({ "id": new_id }),
        Err((code, msg)) => generate_error_message(code, &msg),
    }
}
