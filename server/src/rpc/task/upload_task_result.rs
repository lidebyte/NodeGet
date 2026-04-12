use crate::entity::task;
use crate::rpc::RpcHelper;
use crate::rpc::task::TaskManager;
use crate::token::get::check_token_limit;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::{Permission, Scope, Task};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::task::{TaskEventResponse, TaskEventType};
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::QuerySelect;
use sea_orm::{EntityTrait, Set};
use serde_json::Value;
use serde_json::value::RawValue;
use std::sync::Arc;
use tracing::{debug, error};

pub async fn upload_task_result(
    manager: &Arc<TaskManager>,
    token: String,
    task_response: TaskEventResponse,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        // 1. 先验证权限（fail-fast 原则）
        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        // 解析原始任务类型以获取任务名称（用于权限检查）
        // 注意：这里使用响应中的任务ID来查找任务类型
        let db = <super::TaskRpcImpl as RpcHelper>::get_db()?;

        // 首先仅获取任务类型以进行权限验证
        let task_type_result: Option<(i64, Value)> =
            task::Entity::find_by_id(task_response.task_id.cast_signed())
                .filter(task::Column::Uuid.eq(task_response.agent_uuid))
                .filter(task::Column::Token.eq(task_response.task_token.clone()))
                .select_only()
                .column(task::Column::Id)
                .column(task::Column::TaskEventType)
                .into_tuple()
                .one(db)
                .await
                .map_err(|e| {
                    error!(target: "task", error = %e, "Database query error");
                    NodegetError::DatabaseError(format!("Database query error: {e}"))
                })?;

        let (_, task_event_type_value) = task_type_result.ok_or_else(|| {
            NodegetError::NotFound("Task validation failed: Invalid ID, UUID, or Token".to_owned())
        })?;

        let original_task_type: TaskEventType = serde_json::from_value(task_event_type_value)
            .map_err(|e| {
                NodegetError::SerializationError(format!("Failed to parse original task type: {e}"))
            })?;

        let task_name = original_task_type.task_name();

        // 2. 检查权限
        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(task_response.agent_uuid)],
            vec![Permission::Task(Task::Write(task_name.to_string()))],
        )
        .await?;

        if !is_allowed {
            return Err(NodegetError::PermissionDenied(format!(
                "Permission Denied: Missing Task Write ({task_name}) permission for this Agent"
            ))
            .into());
        }

        // 保存一份完整的 response 用于通知 blocking waiter
        let response_for_notify = task_response.clone();

        // 3. 权限通过后，获取完整任务记录并进行更新
        let task_model = task::Entity::find_by_id(task_response.task_id.cast_signed())
            .filter(task::Column::Uuid.eq(task_response.agent_uuid))
            .filter(task::Column::Token.eq(task_response.task_token.clone()))
            .one(db)
            .await
            .map_err(|e| {
                error!(target: "task", error = %e, "Database query error");
                NodegetError::DatabaseError(format!("Database query error: {e}"))
            })?
            .ok_or_else(|| {
                NodegetError::NotFound(
                    "Task validation failed: Invalid ID, UUID, or Token".to_owned(),
                )
            })?;

        if task_model.success.is_some() {
            return Err(NodegetError::InvalidInput(
                "Task result has already been uploaded".to_owned(),
            )
            .into());
        }

        let error_message = task_response.error_message.map(|v| {
            let json_v = serde_json::to_value(v).unwrap_or(Value::Null);
            match json_v {
                Value::String(s) => s,
                _ => format!("{json_v}"),
            }
        });

        let task_event_result = task_response
            .task_event_result
            .map(|result| {
                serde_json::to_value(result).map_err(|e| {
                    NodegetError::SerializationError(format!(
                        "Failed to serialize task event result: {e}"
                    ))
                })
            })
            .transpose()
            .map_err(|e| NodegetError::SerializationError(e.to_string()))?;

        let update_result = task::Entity::update_many()
            .set(task::ActiveModel {
                timestamp: Set(Some(task_response.timestamp.cast_signed())),
                success: Set(Some(task_response.success)),
                error_message: Set(error_message),
                task_event_result: Set(task_event_result),
                ..Default::default()
            })
            .filter(task::Column::Id.eq(task_response.task_id.cast_signed()))
            .filter(task::Column::Uuid.eq(task_response.agent_uuid))
            .filter(task::Column::Token.eq(task_response.task_token.clone()))
            .filter(task::Column::Success.is_null())
            .exec(db)
            .await
            .map_err(|e| {
                error!(target: "task", error = %e, "Database update error");
                NodegetError::DatabaseError(format!("Database update error: {e}"))
            })?;

        if update_result.rows_affected == 0 {
            return Err(NodegetError::InvalidInput(
                "Task result has already been uploaded".to_owned(),
            )
            .into());
        }

        // 通知 blocking waiter（如果有 create_task_blocking 在等待此 task_id）
        manager
            .notify_blocking_waiter(response_for_notify.task_id, response_for_notify)
            .await;

        debug!(
            target: "task",
            task_id = task_response.task_id,
            auth_type = if token_or_auth.is_auth() { "Auth" } else { "Token" },
            "Task result uploaded"
        );

        let json_str = format!("{{\"id\":{}}}", task_response.task_id);
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
