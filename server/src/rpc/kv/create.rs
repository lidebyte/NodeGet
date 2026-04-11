use crate::kv::create_kv;
use crate::rpc::kv::auth::check_kv_create_permission;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn create(token: String, name: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "rpc", namespace = %name, "Processing create namespace request");

        // 检查创建命名空间的权限
        check_kv_create_permission(&token).await?;

        let kv_store = create_kv(name).await?;

        let json_str = serde_json::to_string(&kv_store).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize KV store: {e}"))
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
