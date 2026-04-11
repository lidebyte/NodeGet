use crate::kv::set_v_to_kv;
use crate::rpc::kv::auth::check_kv_write_permission;
use jsonrpsee::core::RpcResult;
use tracing::debug;
use nodeget_lib::error::NodegetError;
use serde_json::Value;
use serde_json::value::RawValue;

pub async fn set_value(
    token: String,
    namespace: String,
    key: String,
    value: Value,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "rpc", namespace = %namespace, key = %key, "Processing set_value request");

        // 检查写权限
        check_kv_write_permission(&token, &namespace, &key).await?;

        set_v_to_kv(namespace, key, value).await?;

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
