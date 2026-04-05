use crate::DB;
use crate::entity::token;
use crate::token::hash_string;
use nodeget_lib::error::NodegetError;
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::generate_random_string;
use sea_orm::{EntityTrait, Set, TransactionTrait};

async fn insert_new_super_token(
    db: &sea_orm::DatabaseConnection,
) -> anyhow::Result<(String, String)> {
    let token_key = generate_random_string(16);
    let token_secret = generate_random_string(32);
    let full_token = format!("{token_key}:{token_secret}");

    let username = "root".to_string();
    let raw_password = generate_random_string(32);

    let token_hash = hash_string(&token_secret);
    let password_hash = hash_string(&raw_password);

    let super_token_model = token::ActiveModel {
        id: Set(1),
        version: Set(1),
        token_key: Set(token_key),
        token_hash: Set(token_hash),
        time_stamp_from: Set(None),
        time_stamp_to: Set(None),
        token_limit: Set(serde_json::json!([])),
        username: Set(Some(username)),
        password_hash: Set(Some(password_hash)),
    };

    token::Entity::insert(super_token_model)
        .exec(db)
        .await
        .map_err(|e| {
            NodegetError::DatabaseError(format!("Failed to initialize super token: {e}"))
        })?;

    Ok((full_token, raw_password))
}

// 生成超级令牌，如果已存在则返回 None
//
// # 返回值
// 成功时返回 Some((full_token, raw_password))，如果已存在则返回 None，失败时返回错误消息
pub async fn generate_super_token() -> anyhow::Result<Option<(String, String)>> {
    let db = DB.get().ok_or_else(|| {
        NodegetError::DatabaseError("Database connection not initialized".to_string())
    })?;

    // 使用 INSERT OR IGNORE 模式（通过数据库唯一约束）避免 TOCTOU
    // 先尝试插入，如果失败（记录已存在）则返回 None
    match insert_new_super_token(db).await {
        Ok(result) => Ok(Some(result)),
        Err(e) => {
            // 检查是否是唯一约束冲突（记录已存在）
            let error_msg = format!("{e}");
            if error_msg.contains("UNIQUE constraint failed") || error_msg.contains("duplicate key")
            {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

pub async fn roll_super_token() -> anyhow::Result<(String, String)> {
    let db = DB.get().ok_or_else(|| {
        NodegetError::DatabaseError("Database connection not initialized".to_string())
    })?;

    // 使用事务确保删除和插入是原子操作
    // 如果插入失败，删除会回滚，避免锁定
    db.transaction::<_, _, sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            // 删除旧令牌
            token::Entity::delete_by_id(1).exec(txn).await?;

            // 生成新令牌数据
            let token_key = generate_random_string(16);
            let token_secret = generate_random_string(32);
            let token_hash = hash_string(&token_secret);
            let username = "root".to_string();
            let raw_password = generate_random_string(32);
            let password_hash = hash_string(&raw_password);

            // 插入新令牌
            let super_token_model = token::ActiveModel {
                id: sea_orm::Set(1),
                version: sea_orm::Set(1),
                token_key: sea_orm::Set(token_key.clone()),
                token_hash: sea_orm::Set(token_hash),
                time_stamp_from: sea_orm::Set(None),
                time_stamp_to: sea_orm::Set(None),
                token_limit: sea_orm::Set(serde_json::json!([])),
                username: sea_orm::Set(Some(username)),
                password_hash: sea_orm::Set(Some(password_hash)),
            };

            token::Entity::insert(super_token_model).exec(txn).await?;

            Ok((format!("{token_key}:{token_secret}"), raw_password))
        })
    })
    .await
    .map_err(|e| NodegetError::DatabaseError(format!("Transaction failed: {e}")).into())
}

// 检查给定的令牌或认证信息是否为超级令牌
//
// # 参数
// * `token_or_auth` - 令牌或认证信息
//
// # 返回值
// 返回布尔值表示是否为超级令牌，失败时返回错误消息
pub async fn check_super_token(token_or_auth: &TokenOrAuth) -> anyhow::Result<bool> {
    let db = DB.get().ok_or_else(|| {
        NodegetError::DatabaseError("Database connection not initialized".to_owned())
    })?;
    let super_record = token::Entity::find_by_id(1)
        .one(db)
        .await
        .map_err(|e| NodegetError::DatabaseError(format!("Database error: {e}")))?
        .ok_or_else(|| {
            NodegetError::NotFound("Super Token record (ID 1) not found in database".to_owned())
        })?;

    match token_or_auth {
        TokenOrAuth::Token(key, secret) => {
            Ok(key == &super_record.token_key && hash_string(secret) == super_record.token_hash)
        }
        TokenOrAuth::Auth(username, password) => Ok(Some(username.clone())
            == super_record.username
            && Some(hash_string(password)) == super_record.password_hash),
    }
}
