use crate::kv::set_v_to_kv;
use crate::rpc::kv::auth::check_kv_write_permission;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use serde_json::Value;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn set_value(
    token: String,
    namespace: String,
    key: String,
    value: Value,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "kv", namespace = %namespace, key = %key, "Processing set_value request");

        // 检查写权限
        check_kv_write_permission(&token, &namespace, &key).await?;

        set_v_to_kv(namespace, key, value).await?;

        debug!(target: "kv", "set_value completed");

        let json_str = "{\"success\":true}".to_string();

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
