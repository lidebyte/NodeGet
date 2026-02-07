use crate::token;
use crate::token::get::get_token;
use crate::token::super_token::check_super_token;
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use serde_json::Value;
use serde_json::json;

// 删除令牌的方法
//
// # 参数
// * `token` - 认证令牌
// * `target_token_key` - 要删除的目标令牌的 key（可选，仅 SuperToken 可用）
//
// # 返回值
// 返回删除结果
pub async fn delete(token: String, target_token_key: Option<String>) -> Value {
    let process_logic = async {
        // 解析令牌
        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => return Err((101, format!("Failed to parse token: {e}"))),
        };

        let current_token_info = get_token(&token_or_auth).await?;

        // 检查是否为超级令牌
        let is_super_token = check_super_token(&token_or_auth)
            .await
            .map_err(|e| (102, e))?;

        if is_super_token {
            // SuperToken 可以删除任何令牌
            let Some(target_key_to_delete) = target_token_key else {
                    return Err((
                        102,
                        "Target token key is required for SuperToken deletion".to_string(),
                    ));
                };

            // 执行删除操作
            let delete_result = token::delete_token_by_key(target_key_to_delete.clone())
                .await
                .map_err(|e| (103, e.to_string()))?;

            if delete_result.rows_affected > 0 {
                Ok(json!({
                    "success": true,
                    "message": format!("Token {} deleted successfully by SuperToken", target_key_to_delete),
                    "rows_affected": delete_result.rows_affected
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "message": format!("Token {} not found", target_key_to_delete)
                }))
            }
        } else {
            // 普通 Token 只能删除自己的令牌
            let target_key_to_delete = match target_token_key {
                Some(_) => {
                    // 普通 Token 不能删除其他 Token
                    return Err((
                        102,
                        "Insufficient permission to delete other tokens".to_string(),
                    ));
                }
                None => {
                    // 没有提供目标令牌 key，默认删除自己
                    current_token_info.token_key.clone()
                }
            };

            // 执行删除操作（删除自己的令牌）
            let delete_result = token::delete_token_by_key(target_key_to_delete.clone())
                .await
                .map_err(|e| (103, e.to_string()))?;

            if delete_result.rows_affected > 0 {
                Ok(json!({
                    "success": true,
                    "message": "Own token deleted successfully",
                    "rows_affected": delete_result.rows_affected
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "message": "Own token not found"
                }))
            }
        }
    };

    process_logic
        .await
        .unwrap_or_else(|(code, msg)| generate_error_message(code, &msg))
}
