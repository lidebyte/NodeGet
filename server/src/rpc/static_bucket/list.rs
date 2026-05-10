use crate::static_file::list_all_names;
use crate::token::super_token::check_super_token;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::token_auth::TokenOrAuth;
use serde_json::value::RawValue;
use tracing::{debug, warn};

pub async fn list_rpc(token: String) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        debug!(target: "static_bucket", "processing static-bucket_list request");

        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let is_super_token = check_super_token(&token_or_auth)
            .await
            .map_err(|e| NodegetError::PermissionDenied(format!("{e}")))?;

        if !is_super_token {
            warn!(target: "static_bucket", "non-supertoken attempted to list all static names");
            return Err(NodegetError::PermissionDenied(
                "Only SuperToken can list all static names".to_owned(),
            )
            .into());
        }

        let names = list_all_names().await;
        debug!(target: "static_bucket", count = names.len(), "static-bucket_list completed");

        let json_str = serde_json::to_string(&names).map_err(|e| {
            NodegetError::SerializationError(format!("Failed to serialize static name list: {e}"))
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
