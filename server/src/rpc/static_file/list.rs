use crate::rpc::static_file::auth::check_static_permission;
use crate::static_file::list_file;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::StaticFile;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn list_rpc(token: String, name: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "static", name = %name, "processing static_list request");

        check_static_permission(&token, &name, StaticFile::Read).await?;
        debug!(target: "static", name = %name, "static_list permission check passed");

        let files = list_file(&name).await?;

        let json_str = serde_json::to_string(&files).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize file list: {e}"))
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
