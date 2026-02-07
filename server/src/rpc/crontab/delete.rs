use crate::crontab::delete_crontab_by_name;
use crate::token::get::get_token;
use nodeget_lib::permission::data_structure::{Crontab as CrontabPermission, Permission};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use nodeget_lib::utils::get_local_timestamp_ms;
use serde_json::{Value, json};

pub async fn delete(token: String, name: String) -> Value {
    let process_logic = async {
        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => return Err((101, format!("Failed to parse token: {e}"))),
        };

        let token_info = get_token(&token_or_auth).await?;

        let now = get_local_timestamp_ms().cast_signed();

        if let Some(from) = token_info.timestamp_from
            && now < from
        {
            return Err((102, "Token is not yet valid".to_string()));
        }

        if let Some(to) = token_info.timestamp_to
            && now > to
        {
            return Err((102, "Token has expired".to_string()));
        }

        // 检查用户是否有 Crontab::Delete 权限
        let has_crontab_delete_permission = token_info.token_limit.iter().any(|limit| {
            limit
                .permissions
                .iter()
                .any(|perm| matches!(perm, Permission::Crontab(CrontabPermission::Delete)))
        });

        if !has_crontab_delete_permission {
            return Err((
                102,
                "Permission Denied: Insufficient Crontab Delete permission".to_string(),
            ));
        }

        let deleted = delete_crontab_by_name(name)
            .await
            .map_err(|e| (103, e.to_string()))?;

        if deleted {
            Ok(json!({"success": true, "message": "Crontab deleted successfully"}))
        } else {
            Ok(json!({"success": false, "message": "Crontab not found"}))
        }
    };

    process_logic
        .await
        .unwrap_or_else(|(code, msg)| generate_error_message(code, &msg))
}
