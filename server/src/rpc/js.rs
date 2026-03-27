use crate::js_runtime::js_runner;
use jsonrpsee::core::RpcResult;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::error::NodegetError;
use serde_json::value::RawValue;

#[rpc(server, namespace = "js")]
pub trait Rpc {
    #[method(name = "test")]
    async fn test(&self, script: String) -> RpcResult<Box<RawValue>>;
}

pub struct JsRpcImpl;

#[async_trait]
impl RpcServer for JsRpcImpl {
    async fn test(&self, script: String) -> RpcResult<Box<RawValue>> {
        let join = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    NodegetError::Other(format!("Failed to build JS runtime host: {e}"))
                })?;

            rt.block_on(js_runner(script))
                .map_err(|e| {
                    NodegetError::Other(format!("JavaScript runtime execution failed: {e}"))
                })
                .and_then(|json_value| {
                    let json_str = serde_json::to_string(&json_value)
                        .map_err(|e| NodegetError::SerializationError(e.to_string()))?;
                    RawValue::from_string(json_str)
                        .map_err(|e| NodegetError::SerializationError(e.to_string()))
                })
        });

        let output = join.await.map_err(|e| {
            let nodeget_err =
                NodegetError::Other(format!("JavaScript runtime task join failed: {e}"));
            jsonrpsee::types::ErrorObject::owned(
                nodeget_err.error_code() as i32,
                nodeget_err.to_string(),
                None::<()>,
            )
        })?;

        output.map_err(|nodeget_err| {
            jsonrpsee::types::ErrorObject::owned(
                nodeget_err.error_code() as i32,
                nodeget_err.to_string(),
                None::<()>,
            )
        })
    }
}
