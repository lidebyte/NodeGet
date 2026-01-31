use crate::entity::task;
use crate::rpc::RpcHelper;
use crate::rpc::task::TaskRpcImpl;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use uuid::Uuid;

pub async fn check_agent(
    agent_uuid: String,
    task_token: String,
    task_id: u64,
) -> Result<bool, (i64, String)> {
    let agent_uuid =
        Uuid::parse_str(&agent_uuid).map_err(|_| (101, "Invalid Agent UUID format".to_string()))?;

    // 稍微借用下 Task Rpc 的 get_db
    let db = TaskRpcImpl::get_db()?;

    let id = task_id.cast_signed();

    let count = task::Entity::find()
        .filter(task::Column::Id.eq(id)) // 匹配 Task ID
        .filter(task::Column::Uuid.eq(agent_uuid)) // 匹配 Agent UUID
        .filter(task::Column::Token.eq(task_token)) // 匹配 Task Token
        .filter(task::Column::TaskEventResult.is_null()) //  匹配未完成
        .count(db)
        .await
        .map_err(|e| (103, format!("Database error: {e}")))?;

    Ok(count > 0)
}
