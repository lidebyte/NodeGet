use crate::entity::metadata as metadata_entity;
use crate::rpc::RpcHelper;
use crate::token::get::check_token_limit;
use nodeget_lib::metadata;
use nodeget_lib::permission::data_structure::{Metadata as MetadataPermission, Permission, Scope};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::generate_error_message;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use uuid::Uuid;

pub async fn get(token: String, uuid: Uuid) -> Value {
    let process_logic = async {
        let token_or_auth = match TokenOrAuth::from_full_token(&token) {
            Ok(toa) => toa,
            Err(e) => {
                return Err((101, format!("Failed to parse token: {e}")));
            }
        };

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(uuid)],
            vec![Permission::Metadata(MetadataPermission::Read)],
        )
        .await?;

        if !is_allowed {
            return Err((
                102,
                "Permission Denied: Missing Metadata Read permission".to_string(),
            ));
        }

        let db = <super::MetadataRpcImpl as RpcHelper>::get_db()?;

        match metadata_entity::Entity::find()
            .filter(metadata_entity::Column::Uuid.eq(uuid))
            .one(db)
            .await
        {
            Ok(Some(model)) => {
                let metadata_struct = metadata::Metadata {
                    agent_uuid: model.uuid,
                    agent_name: model.name,
                    agent_tags: model.tags.map_or_else(std::vec::Vec::new, |json_val| {
                        serde_json::from_value(json_val).unwrap_or_else(|_| vec![])
                    }),
                };
                Ok(metadata_struct)
            }
            Ok(None) => {
                let empty_metadata = metadata::Metadata {
                    agent_uuid: uuid,
                    agent_name: String::new(),
                    agent_tags: vec![],
                };
                Ok(empty_metadata)
            }
            _ => Err((103, "Database error".to_string())),
        }
    };

    match process_logic.await {
        Ok(metadata) => serde_json::to_value(metadata).unwrap_or_else(|_| serde_json::json!({})),
        Err((code, msg)) => generate_error_message(code, &msg),
    }
}
