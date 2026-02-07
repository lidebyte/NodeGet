use crate::entity::metadata as metadata_entity;
use crate::rpc::RpcHelper;
use crate::token::get::check_token_limit;
use nodeget_lib::metadata;
use nodeget_lib::permission::data_structure::{Metadata as MetadataPermission, Permission, Scope};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter,
};
use serde_json::Value;

pub async fn write(token: String, metadata: metadata::Metadata) -> Value {
    let process_logic = async {
        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => {
                return Err((101, format!("Failed to parse token: {e}")));
            }
        };

        let is_allowed = match check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(metadata.agent_uuid)],
            vec![Permission::Metadata(MetadataPermission::Write)],
        )
        .await
        {
            Ok(result) => result,
            Err((code, msg)) => {
                return Err((code, msg));
            }
        };

        if !is_allowed {
            return Err((
                102,
                "Permission Denied: Missing Metadata Write permission".to_string(),
            ));
        }

        let db = <super::MetadataRpcImpl as RpcHelper>::get_db()?;

        let tags_json = match serde_json::to_value(&metadata.agent_tags) {
            Ok(json_val) => Some(json_val),
            Err(e) => {
                return Err((101, format!("Failed to serialize tags: {e}")));
            }
        };

        match metadata_entity::Entity::find()
            .filter(metadata_entity::Column::Uuid.eq(metadata.agent_uuid))
            .one(db)
            .await
        {
            Ok(Some(existing_model)) => {
                let mut active_model: metadata_entity::ActiveModel = existing_model.into();
                active_model.name = Set(metadata.agent_name);
                active_model.tags = Set(tags_json);

                match active_model.clone().update(db).await {
                    Ok(_) => Ok(active_model.id.unwrap()),
                    Err(e) => Err((103, format!("Database update error: {e}"))),
                }
            }
            Ok(None) => {
                let new_metadata = metadata_entity::ActiveModel {
                    id: ActiveValue::default(),
                    uuid: Set(metadata.agent_uuid),
                    name: Set(metadata.agent_name),
                    tags: Set(tags_json),
                };

                match new_metadata.insert(db).await {
                    Ok(inserted_model) => Ok(inserted_model.id),
                    Err(e) => Err((103, format!("Database insert error: {e}"))),
                }
            }
            _ => Err((103, "Database error".to_string())),
        }
    };

    match process_logic.await {
        Ok(id) => serde_json::json!({"id": id}),
        Err((code, msg)) => generate_error_message(code, &msg),
    }
}
