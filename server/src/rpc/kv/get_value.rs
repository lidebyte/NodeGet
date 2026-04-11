use crate::kv::get_v_from_kv;
use crate::rpc::kv::auth::check_kv_read_permission;
use jsonrpsee::core::RpcResult;
use tracing::debug;
use nodeget_lib::error::NodegetError;
use serde_json::value::RawValue;

pub async fn get_value(token: String, namespace: String, key: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "rpc", namespace = %namespace, key = %key, "Processing get_value request");

        // 检查读权限
        check_kv_read_permission(&token, &namespace, &key).await?;

        let value = get_v_from_kv(namespace, key).await?;

        let json_str = match value {
            Some(v) => serde_json::to_string(&v).map_err(|e| {
                NodegetError::SerializationError(format!("Failed to serialize value: {e}"))
            })?,
            None => "null".to_string(),
        };

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
