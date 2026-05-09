use crate::rpc::static_file::auth::check_static_permission;
use crate::static_file::upload_file;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::StaticFile;
use serde_json::value::RawValue;
use tracing::debug;

pub async fn upload_file_rpc(
    token: String,
    name: String,
    path: String,
    body: Option<Vec<u8>>,
    base64: Option<String>,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "static", name = %name, path = %path, "processing static_upload_file request");

        check_static_permission(&token, &name, StaticFile::Write).await?;
        debug!(target: "static", name = %name, "static_upload_file permission check passed");

        upload_file(&name, &path, body, base64).await?;

        let json_str = r#"{"success":true}"#.to_string();

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
