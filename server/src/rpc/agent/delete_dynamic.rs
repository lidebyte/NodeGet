use crate::entity::dynamic_monitoring;
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use jsonrpsee::core::RpcResult;
use nodeget_lib::error::NodegetError;
use nodeget_lib::monitoring::query::QueryCondition;
use nodeget_lib::permission::data_structure::{DynamicMonitoring, Permission, Scope};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use sea_orm::{ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::value::RawValue;
use std::collections::HashSet;
use tracing::{debug, error};

pub async fn delete_dynamic(
    token: String,
    conditions: Vec<QueryCondition>,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let scopes = scopes_from_conditions(&conditions);
        let is_allowed = check_token_limit(
            &token_or_auth,
            scopes,
            vec![Permission::DynamicMonitoring(DynamicMonitoring::Delete)],
        )
        .await?;

        if !is_allowed {
            return Err(NodegetError::PermissionDenied(
                "Permission Denied: Missing DynamicMonitoring Delete permission for requested scope"
                    .to_owned(),
            )
                .into());
        }

        let db = AgentRpcImpl::get_db()?;
        let (limit_count, is_last) = extract_limit_and_last(&conditions);

        let rows_affected = if is_last || limit_count.is_some() {
            let mut query = dynamic_monitoring::Entity::find();
            for cond in &conditions {
                match cond {
                    QueryCondition::Uuid(uuid) => {
                        query = query.filter(dynamic_monitoring::Column::Uuid.eq(*uuid));
                    }
                    QueryCondition::TimestampFromTo(start, end) => {
                        query = query.filter(
                            dynamic_monitoring::Column::Timestamp
                                .gte(*start)
                                .and(dynamic_monitoring::Column::Timestamp.lte(*end)),
                        );
                    }
                    QueryCondition::TimestampFrom(start) => {
                        query = query.filter(dynamic_monitoring::Column::Timestamp.gte(*start));
                    }
                    QueryCondition::TimestampTo(end) => {
                        query = query.filter(dynamic_monitoring::Column::Timestamp.lte(*end));
                    }
                    QueryCondition::Limit(_) | QueryCondition::Last => {}
                }
            }

            let limit = if is_last { 1 } else { limit_count.unwrap_or(0) };
            let ids: Vec<i64> = query
                .select_only()
                .column(dynamic_monitoring::Column::Id)
                .order_by_desc(dynamic_monitoring::Column::Timestamp)
                .limit(limit)
                .into_tuple()
                .all(db)
                .await
                .map_err(|e| {
                    error!(target: "monitoring", error = %e, "Database query error");
                    NodegetError::DatabaseError(format!("Database query error: {e}"))
                })?;

            if ids.is_empty() {
                0
            } else {
                dynamic_monitoring::Entity::delete_many()
                    .filter(dynamic_monitoring::Column::Id.is_in(ids))
                    .exec(db)
                    .await
                    .map_err(|e| {
                        error!(target: "monitoring", error = %e, "Database delete error");
                        NodegetError::DatabaseError(format!("Database delete error: {e}"))
                    })?
                    .rows_affected
            }
        } else {
            let mut query = dynamic_monitoring::Entity::delete_many();
            for cond in &conditions {
                match cond {
                    QueryCondition::Uuid(uuid) => {
                        query = query.filter(dynamic_monitoring::Column::Uuid.eq(*uuid));
                    }
                    QueryCondition::TimestampFromTo(start, end) => {
                        query = query.filter(
                            dynamic_monitoring::Column::Timestamp
                                .gte(*start)
                                .and(dynamic_monitoring::Column::Timestamp.lte(*end)),
                        );
                    }
                    QueryCondition::TimestampFrom(start) => {
                        query = query.filter(dynamic_monitoring::Column::Timestamp.gte(*start));
                    }
                    QueryCondition::TimestampTo(end) => {
                        query = query.filter(dynamic_monitoring::Column::Timestamp.lte(*end));
                    }
                    QueryCondition::Limit(_) | QueryCondition::Last => {}
                }
            }
            query
                .exec(db)
                .await
                .map_err(|e| {
                    error!(target: "monitoring", error = %e, "Database delete error");
                    NodegetError::DatabaseError(format!("Database delete error: {e}"))
                })?
                .rows_affected
        };

        debug!(target: "monitoring", rows_affected = rows_affected, conditions = conditions.len(), "Dynamic monitoring delete completed");

        let json_str = format!(
            "{{\"success\":true,\"deleted\":{},\"condition_count\":{}}}",
            rows_affected,
            conditions.len()
        );
        RawValue::from_string(json_str)
            .map_err(|e| NodegetError::SerializationError(e.to_string()).into())
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

fn scopes_from_conditions(conditions: &[QueryCondition]) -> Vec<Scope> {
    let mut seen = HashSet::new();
    let mut scopes = Vec::new();

    for cond in conditions {
        if let QueryCondition::Uuid(uuid) = cond
            && seen.insert(*uuid)
        {
            scopes.push(Scope::AgentUuid(*uuid));
        }
    }

    if scopes.is_empty() {
        scopes.push(Scope::Global);
    }

    scopes
}

fn extract_limit_and_last(conditions: &[QueryCondition]) -> (Option<u64>, bool) {
    let mut limit_count = None;
    let mut is_last = false;

    for cond in conditions {
        match cond {
            QueryCondition::Limit(n) => {
                limit_count = Some(*n);
            }
            QueryCondition::Last => {
                is_last = true;
            }
            QueryCondition::Uuid(_)
            | QueryCondition::TimestampFromTo(_, _)
            | QueryCondition::TimestampFrom(_)
            | QueryCondition::TimestampTo(_) => {}
        }
    }

    (limit_count, is_last)
}
