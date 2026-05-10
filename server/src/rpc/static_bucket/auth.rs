use crate::token::get::check_token_limit;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::{
    Permission, Scope, StaticBucket as StaticBucketPermission,
};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use tracing::{trace, warn};

pub async fn check_static_bucket_permission(
    token: &str,
    name: &str,
    permission: StaticBucketPermission,
) -> anyhow::Result<()> {
    trace!(target: "static_bucket", name = %name, permission = ?permission, "checking static-bucket permission");
    let token_or_auth = TokenOrAuth::from_full_token(token)
        .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

    let permission_name = format!("{permission:?}");
    let is_allowed = check_token_limit(
        &token_or_auth,
        vec![Scope::StaticBucket(name.to_owned())],
        vec![Permission::StaticBucket(permission)],
    )
    .await?;

    if is_allowed {
        return Ok(());
    }

    warn!(target: "static_bucket", name = %name, permission = %permission_name, "permission denied");
    Err(NodegetError::PermissionDenied(format!(
        "Permission denied for static-bucket '{name}', required permission: {permission_name}"
    ))
    .into())
}
