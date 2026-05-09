use crate::rpc::static_file::auth::check_static_permission;
use crate::static_file::update_static;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::StaticFile;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn update(
    token: String,
    name: String,
    path: String,
    is_http_root: bool,
    cors: bool,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "static", name = %name, "processing static_update request");

        check_static_permission(&token, &name, StaticFile::Write).await?;
        debug!(target: "static", name = %name, "static_update permission check passed");

        let model = update_static(name, path, is_http_root, cors).await?;

        let json_str = serde_json::to_string(&model).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize static: {e}"))
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
