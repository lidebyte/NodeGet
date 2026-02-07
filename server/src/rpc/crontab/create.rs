use crate::entity::crontab;
use crate::rpc::RpcHelper;
use crate::rpc::crontab::CrontabRpcImpl;
use crate::token::get::check_token_limit;
use cron::Schedule;
use nodeget_lib::crontab::{AgentCronType, CronType};
use nodeget_lib::permission::data_structure::{
    Crontab as CrontabPermission, Permission, Scope, Task,
};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::task::TaskEventType;
use nodeget_lib::utils::error_message::generate_error_message;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use std::str::FromStr;

pub async fn create(
    token: String,
    name: String,
    cron_expression: String,
    cron_type: CronType,
) -> Value {
    let process_logic = async {
        if let Err(e) = Schedule::from_str(&cron_expression) {
            return Err((101, format!("Invalid cron expression: {e}")));
        }

        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => return Err((101, format!("Failed to parse token: {e}"))),
        };

        let mut scopes = Vec::new();
        let mut permissions = Vec::new();

        permissions.push(Permission::Crontab(CrontabPermission::Write));

        match &cron_type {
            CronType::Agent(uuids, agent_cron_type) => {
                if uuids.is_empty() {
                    return Err((101, "Agent list cannot be empty".to_string()));
                }
                for uuid in uuids {
                    scopes.push(Scope::AgentUuid(*uuid));
                }

                match agent_cron_type {
                    AgentCronType::Task(task_event_type) => {
                        let task_name = match task_event_type {
                            TaskEventType::Ping(_) => "ping",
                            TaskEventType::TcpPing(_) => "tcp_ping",
                            TaskEventType::HttpPing(_) => "http_ping",
                            TaskEventType::WebShell(_) => "web_shell",
                            TaskEventType::Execute(_) => "execute",
                            TaskEventType::Ip => "ip",
                        };
                        permissions.push(Permission::Task(Task::Create(task_name.to_string())));
                    }
                }
            }
            CronType::Server(_) => {
                scopes.push(Scope::Global);
            }
        }

        let is_allowed = check_token_limit(&token_or_auth, scopes, permissions).await?;
        if !is_allowed {
            return Err((
                102,
                "Permission Denied: Insufficient Crontab or Task permissions".to_string(),
            ));
        }

        let db = CrontabRpcImpl::get_db()?;

        let existing_job = crontab::Entity::find()
            .filter(crontab::Column::Name.eq(&name))
            .one(db)
            .await
            .map_err(|e| (103, e.to_string()))?;

        let cron_type_json = CrontabRpcImpl::try_set_json(&cron_type).map_err(|e| (101, e))?;

        let res_id = if let Some(model) = existing_job {
            // 更新现有任务
            let mut active_model: crontab::ActiveModel = model.into();
            active_model.cron_expression = Set(cron_expression);
            active_model.cron_type = cron_type_json;
            active_model.enable = Set(true);

            let updated = active_model
                .update(db)
                .await
                .map_err(|e| (103, e.to_string()))?;
            updated.id
        } else {
            // 创建新任务
            let new_model = crontab::ActiveModel {
                id: ActiveValue::NotSet,
                name: Set(name),
                enable: Set(true),
                cron_expression: Set(cron_expression),
                cron_type: cron_type_json,
                last_run_time: Set(None),
            };
            let inserted = crontab::Entity::insert(new_model)
                .exec(db)
                .await
                .map_err(|e| (103, e.to_string()))?;
            inserted.last_insert_id
        };

        Ok(res_id)
    };

    match process_logic.await {
        Ok(id) => json!({ "id": id }),
        Err((code, msg)) => generate_error_message(code, &msg),
    }
}
