use crate::DB;
use crate::entity::crontab_result;
use crate::rpc::crontab_result::CrontabResultDelete;
use crate::rpc::crontab_result::auth::check_crontab_result_delete_permission;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::value::RawValue;
use tracing::debug;

pub async fn delete(token: String, delete_params: CrontabResultDelete) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "crontab_result", before_time = delete_params.before_time, cron_name = ?delete_params.cron_name, "processing crontab_result delete request");
        let db = DB
            .get()
            .ok_or_else(|| NodegetError::DatabaseError("DB not initialized".to_owned()))?;

        // 检查删除权限
        check_crontab_result_delete_permission(&token, delete_params.cron_name.as_deref()).await?;

        // 构建删除条件
        let mut delete = crontab_result::Entity::delete_many()
            .filter(crontab_result::Column::RunTime.lt(delete_params.before_time));

        // 如果指定了 cron_name，添加过滤
        if let Some(ref cron_name) = delete_params.cron_name {
            delete = delete.filter(crontab_result::Column::CronName.eq(cron_name.clone()));
        }

        // 执行删除
        let result = delete.exec(db).await.map_err(|e| {
            NodegetError::DatabaseError(format!("Failed to delete crontab_result: {e}"))
        })?;

        let deleted_count = result.rows_affected;

        debug!(target: "crontab_result", deleted_count, "crontab_result delete completed");

        // 构建响应
        let response = serde_json::json!({
            "success": true,
            "deleted_count": deleted_count,
        });

        let json_str = serde_json::to_string(&response).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize response: {e}"))
        })?;

        RawValue::from_string(json_str)
            .map_err(|e| NodegetError::SerializationError(format!("{e}")).into())
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
