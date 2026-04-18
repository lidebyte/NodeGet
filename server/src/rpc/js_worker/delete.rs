use crate::entity::js_worker;
use crate::js_runtime::runtime_pool;
use crate::rpc::RpcHelper;
use crate::rpc::js_worker::JsWorkerRpcImpl;
use crate::rpc::js_worker::auth::check_js_worker_permission;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::JsWorker as JsWorkerPermission;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::value::RawValue;
use tracing::{debug, trace};

pub async fn delete(token: String, name: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        if name.trim().is_empty() {
            return Err(NodegetError::InvalidInput("name cannot be empty".to_owned()).into());
        }
        debug!(target: "js_worker", name = %name, "processing js_worker delete request");

        check_js_worker_permission(&token, name.as_str(), JsWorkerPermission::Delete).await?;

        debug!(target: "js_worker", name = %name, "js_worker delete permission check passed");

        let db = JsWorkerRpcImpl::get_db()?;
        let delete_result = js_worker::Entity::delete_many()
            .filter(js_worker::Column::Name.eq(name.as_str()))
            .exec(db)
            .await
            .map_err(|e| NodegetError::DatabaseError(e.to_string()))?;

        if delete_result.rows_affected == 0 {
            return Err(NodegetError::NotFound(format!("js_worker not found: {name}")).into());
        }
        runtime_pool::global_pool().evict_worker(name.as_str());
        trace!(target: "js_worker", name = %name, "evicted worker from runtime pool after delete");

        debug!(target: "js_worker", name = %name, rows_affected = delete_result.rows_affected, "js_worker deleted successfully");

        let response = serde_json::json!({
            "success": true,
            "rows_affected": delete_result.rows_affected
        });
        let json_str = serde_json::to_string(&response)
            .map_err(|e| NodegetError::SerializationError(e.to_string()))?;
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
