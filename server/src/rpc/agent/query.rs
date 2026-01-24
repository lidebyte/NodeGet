use jsonrpsee::core::RpcResult;
use crate::entity::{dynamic_monitoring, static_monitoring};
use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use futures::StreamExt;
use log::error;
use nodeget_lib::monitoring::query::{
    DynamicDataQuery, DynamicDataQueryField, QueryCondition, StaticDataQuery,
    StaticDataQueryField,
};
use nodeget_lib::utils::error_message::error_to_raw;
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Map, Value};
use serde_json::value::RawValue;
use migration::ExprTrait;

pub async fn query_static(_token: String, static_data_query: StaticDataQuery) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        let db = AgentRpcImpl::get_db()?;

        let mut query = static_monitoring::Entity::find().select_only();

        query = query
            .column(static_monitoring::Column::Uuid)
            .column(static_monitoring::Column::Timestamp);

        for field in &static_data_query.fields {
            match field {
                StaticDataQueryField::Cpu => query = query.column(static_monitoring::Column::CpuData),
                StaticDataQueryField::System => query = query.column(static_monitoring::Column::SystemData),
                StaticDataQueryField::Gpu => query = query.column(static_monitoring::Column::GpuData),
            }
        }

        let mut is_last = false;
        let mut limit_count: Option<u64> = None;

        for cond in static_data_query.condition {
            match cond {
                QueryCondition::Uuid(uuid) => {
                    query = query.filter(static_monitoring::Column::Uuid.eq(uuid));
                }
                QueryCondition::TimestampFromTo(start, end) => {
                    query = query.filter(
                        static_monitoring::Column::Timestamp
                            .gte(start)
                            .and(static_monitoring::Column::Timestamp.lte(end)),
                    );
                }
                QueryCondition::TimestampFrom(start) => {
                    query = query.filter(static_monitoring::Column::Timestamp.gte(start));
                }
                QueryCondition::TimestampTo(end) => {
                    query = query.filter(static_monitoring::Column::Timestamp.lte(end));
                }
                QueryCondition::Limit(n) => {
                    limit_count = Some(n);
                }
                QueryCondition::Last => {
                    is_last = true;
                }
            }
        }

        if let Some(l) = limit_count {
            query = query
                .order_by(static_monitoring::Column::Timestamp, Order::Desc)
                .limit(l);
        } else {
            query = query.order_by(static_monitoring::Column::Timestamp, Order::Asc);
        }

        if is_last {
            query = query
                .order_by(static_monitoring::Column::Timestamp, Order::Desc)
                .limit(1);
        } else {
            query = query.order_by(static_monitoring::Column::Timestamp, Order::Asc);
        }

        let mut stream = query.into_json().stream(db).await.map_err(|e| {
            error!("Database query error: {e}");
            (103, format!("Database query error: {e}"))
        })?;

        let capacity = limit_count.unwrap_or(100) as usize * 200;
        let mut output_buffer: Vec<u8> = Vec::with_capacity(capacity);

        output_buffer.push(b'[');

        let mut first = true;

        while let Some(item_res) = stream.next().await {
            match item_res {
                Ok(mut v) => {
                    if let Some(obj) = v.as_object_mut() {
                        rename_key(obj, "cpu_data", "cpu");
                        rename_key(obj, "system_data", "system");
                        rename_key(obj, "gpu_data", "gpu");
                    }

                    if !first {
                        output_buffer.push(b',');
                    } else {
                        first = false;
                    }

                    if let Err(e) = serde_json::to_writer(&mut output_buffer, &v) {
                        error!("Serialization failed: {e}");
                        return Err((101, format!("Serialization failed: {e}")));
                    }
                },
                Err(e) => {
                    error!("Stream read error: {e}");
                    return Err((103, format!("Stream read error: {e}")));
                }
            }
        }

        output_buffer.push(b']');

        let json_string = String::from_utf8(output_buffer).map_err(|e| {
            error!("UTF8 conversion error: {e}");
            (101, "UTF8 conversion error (internal)".to_string())
        })?;

        let raw_value = RawValue::from_string(json_string).map_err(|e| {
            error!("RawValue creation error: {e}");
            (101, "RawValue creation error".to_string())
        })?;

        Ok(raw_value)
    };

    Ok(process_logic
        .await
        .unwrap_or_else(|(code, msg)| error_to_raw(code, &msg)))
}

pub async fn query_dynamic(_token: String, dynamic_data_query: DynamicDataQuery) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        let db = AgentRpcImpl::get_db()?;

        let mut query = dynamic_monitoring::Entity::find().select_only();

        query = query
            .column(dynamic_monitoring::Column::Uuid)
            .column(dynamic_monitoring::Column::Timestamp);

        for field in &dynamic_data_query.fields {
            match field {
                DynamicDataQueryField::Cpu => query = query.column(dynamic_monitoring::Column::CpuData),
                DynamicDataQueryField::Ram => query = query.column(dynamic_monitoring::Column::RamData),
                DynamicDataQueryField::Load => query = query.column(dynamic_monitoring::Column::LoadData),
                DynamicDataQueryField::System => query = query.column(dynamic_monitoring::Column::SystemData),
                DynamicDataQueryField::Disk => query = query.column(dynamic_monitoring::Column::DiskData),
                DynamicDataQueryField::Network => query = query.column(dynamic_monitoring::Column::NetworkData),
                DynamicDataQueryField::Gpu => query = query.column(dynamic_monitoring::Column::GpuData),
            }
        }

        let mut is_last = false;
        let mut limit_count: Option<u64> = None;

        for cond in dynamic_data_query.condition {
            match cond {
                QueryCondition::Uuid(uuid) => {
                    query = query.filter(dynamic_monitoring::Column::Uuid.eq(uuid));
                }
                QueryCondition::TimestampFromTo(start, end) => {
                    query = query.filter(
                        dynamic_monitoring::Column::Timestamp
                            .gte(start)
                            .and(dynamic_monitoring::Column::Timestamp.lte(end)),
                    );
                }
                QueryCondition::TimestampFrom(start) => {
                    query = query.filter(dynamic_monitoring::Column::Timestamp.gte(start));
                }
                QueryCondition::TimestampTo(end) => {
                    query = query.filter(dynamic_monitoring::Column::Timestamp.lte(end));
                }
                QueryCondition::Limit(n) => {
                    limit_count = Some(n);
                }
                QueryCondition::Last => {
                    is_last = true;
                }
            }
        }

        if let Some(l) = limit_count {
            query = query
                .order_by(dynamic_monitoring::Column::Timestamp, Order::Desc)
                .limit(l);
        } else {
            query = query.order_by(dynamic_monitoring::Column::Timestamp, Order::Asc);
        }

        if is_last {
            query = query
                .order_by(dynamic_monitoring::Column::Timestamp, Order::Desc)
                .limit(1);
        } else {
            query = query.order_by(dynamic_monitoring::Column::Timestamp, Order::Asc);
        }

        let mut stream = query.into_json().stream(db).await.map_err(|e| {
            error!("Database query error: {e}");
            (103, format!("Database query error: {e}"))
        })?;

        let capacity = limit_count.unwrap_or(5000) as usize * 200;
        let mut output_buffer: Vec<u8> = Vec::with_capacity(capacity);

        output_buffer.push(b'[');
        let mut first = true;

        while let Some(item_res) = stream.next().await {
            match item_res {
                Ok(mut v) => {
                    if let Some(obj) = v.as_object_mut() {
                        rename_key(obj, "cpu_data", "cpu");
                        rename_key(obj, "ram_data", "ram");
                        rename_key(obj, "load_data", "load");
                        rename_key(obj, "system_data", "system");
                        rename_key(obj, "disk_data", "disk");
                        rename_key(obj, "network_data", "network");
                        rename_key(obj, "gpu_data", "gpu");
                    }

                    if !first {
                        output_buffer.push(b',');
                    } else {
                        first = false;
                    }

                    if let Err(e) = serde_json::to_writer(&mut output_buffer, &v) {
                        error!("Serialization failed: {e}");
                        return Err((101, format!("Serialization failed: {e}")));
                    }
                },
                Err(e) => {
                    error!("Stream read error: {e}");
                    return Err((103, format!("Stream read error: {e}")));
                }
            }
        }

        output_buffer.push(b']');

        let json_string = String::from_utf8(output_buffer).map_err(|e| {
            error!("UTF8 conversion error: {e}");
            (101, "UTF8 conversion error".to_string())
        })?;

        let raw_value = RawValue::from_string(json_string).map_err(|e| {
            error!("RawValue creation error: {e}");
            (101, "RawValue creation error".to_string())
        })?;

        Ok(raw_value)
        // --- 核心优化结束 ---
    };

    Ok(process_logic
        .await
        .unwrap_or_else(|(code, msg)| error_to_raw(code, &msg)))
}

fn rename_key(map: &mut Map<String, Value>, old_key: &str, new_key: &str) {
    if let Some(v) = map.remove(old_key) {
        map.insert(new_key.to_string(), v);
    }
}
