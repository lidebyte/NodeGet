use crate::kv::get_keys_from_kv;
use crate::rpc::kv::auth::check_kv_list_keys_permission;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn get_all_keys(token: String, namespace: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "kv", namespace = %namespace, "Processing get_all_keys request");

        // 检查列出 keys 的权限
        check_kv_list_keys_permission(&token, &namespace).await?;

        let keys = get_keys_from_kv(namespace).await?;

        debug!(target: "kv", keys_count = keys.len(), "get_all_keys completed");

        let json_str = serde_json::to_string(&keys).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize keys: {e}"))
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
