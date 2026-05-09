use crate::token::get::check_token_limit;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::data_structure::{
    Permission, Scope, StaticFile as StaticFilePermission,
};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use tracing::{trace, warn};

pub async fn check_static_permission(
    token: &str,
    name: &str,
    permission: StaticFilePermission,
) -> anyhow::Result<()> {
    trace!(target: "static", name = %name, permission = ?permission, "checking static permission");
    let token_or_auth = TokenOrAuth::from_full_token(token)
        .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

    let permission_name = format!("{permission:?}");
    let is_allowed = check_token_limit(
        &token_or_auth,
        vec![Scope::StaticFile(name.to_owned())],
        vec![Permission::StaticFile(permission)],
    )
    .await?;

    if is_allowed {
        return Ok(());
    }

    warn!(target: "static", name = %name, permission = %permission_name, "permission denied");
    Err(NodegetError::PermissionDenied(format!(
        "Permission denied for static '{name}', required permission: {permission_name}"
    ))
    .into())
}
