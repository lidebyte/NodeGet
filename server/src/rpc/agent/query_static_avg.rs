use crate::rpc::RpcHelper;
use crate::rpc::agent::AgentRpcImpl;
use crate::token::get::check_token_limit;
use jsonrpsee::core::RpcResult;
use tracing::error;
use nodeget_lib::error::NodegetError;
use nodeget_lib::monitoring::query::{StaticDataAvgQuery, StaticDataQueryField};
use nodeget_lib::permission::data_structure::{Permission, Scope, StaticMonitoring};
use nodeget_lib::permission::token_auth::TokenOrAuth;
use nodeget_lib::utils::error_message::anyhow_error_to_raw;
use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};
use serde_json::Value;
use serde_json::value::RawValue;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

// 生成唯一错误ID用于内部追踪
static ERROR_COUNTER: AtomicU64 = AtomicU64::new(0);
fn generate_error_id() -> u64 {
    ERROR_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, FromQueryResult)]
struct JsonAggRow {
    data: Value,
}

pub async fn query_static_avg(
    token: String,
    static_data_avg_query: StaticDataAvgQuery,
) -> RpcResult<Box<RawValue>> {
    let process_logic = async {
        validate_avg_query(&static_data_avg_query)?;

        let token_or_auth = TokenOrAuth::from_full_token(&token)
            .map_err(|e| NodegetError::ParseError(format!("Failed to parse token: {e}")))?;

        let permissions: Vec<Permission> = static_data_avg_query
            .fields
            .iter()
            .map(|field| Permission::StaticMonitoring(StaticMonitoring::Read(*field)))
            .collect();

        let is_allowed = check_token_limit(
            &token_or_auth,
            vec![Scope::AgentUuid(static_data_avg_query.uuid)],
            permissions,
        )
        .await?;

        if !is_allowed {
            return Err(NodegetError::PermissionDenied(
                "Permission Denied: Insufficient StaticMonitoring Read permissions".to_owned(),
            )
            .into());
        }

        let db = AgentRpcImpl::get_db()?;
        ensure_postgres_backend(db)?;
        query_static_avg_postgres(db, &static_data_avg_query).await
    };

    match process_logic.await {
        Ok(result) => Ok(result),
        Err(e) => {
            let raw = anyhow_error_to_raw(&e).unwrap_or_else(|_| {
                RawValue::from_string(
                    r#"{"error_id":999,"error_message":"Internal error"}"#.to_owned(),
                )
                .unwrap_or_else(|_| RawValue::from_string("null".to_owned()).unwrap())
            });
            let nodeget_err = nodeget_lib::error::anyhow_to_nodeget_error(&e);
            let json_str = raw.get();
            Err(jsonrpsee::types::ErrorObject::owned(
                nodeget_err.error_code() as i32,
                format!("{nodeget_err}"),
                Some(json_str),
            ))
        }
    }
}

fn validate_avg_query(query: &StaticDataAvgQuery) -> anyhow::Result<()> {
    if query.fields.is_empty() {
        return Err(NodegetError::InvalidInput(
            "fields cannot be empty for static_data_avg_query".to_owned(),
        )
        .into());
    }
    if query.points == 0 {
        return Err(NodegetError::InvalidInput("points must be >= 1".to_owned()).into());
    }
    if let (Some(start), Some(end)) = (query.timestamp_from, query.timestamp_to)
        && start > end
    {
        return Err(NodegetError::InvalidInput(
            "timestamp_from cannot be greater than timestamp_to".to_owned(),
        )
        .into());
    }
    Ok(())
}

fn ensure_postgres_backend(db: &DatabaseConnection) -> anyhow::Result<()> {
    if db.get_database_backend() == DatabaseBackend::Postgres {
        return Ok(());
    }

    Err(NodegetError::InvalidInput(
        "agent_query_static_avg currently only supports PostgreSQL; SQLite and other databases are disabled for this method"
            .to_owned(),
    )
        .into())
}

async fn query_static_avg_postgres(
    db: &DatabaseConnection,
    query: &StaticDataAvgQuery,
) -> anyhow::Result<Box<RawValue>> {
    let sql = build_postgres_static_avg_sql(&query.fields);
    let statement = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        [
            query.uuid.to_string().into(),
            query.timestamp_from.into(),
            query.timestamp_to.into(),
            i64::try_from(query.points)
                .map_err(|_| NodegetError::InvalidInput("points is too large".to_owned()))?
                .into(),
        ],
    );

    let row = JsonAggRow::find_by_statement(statement)
        .one(db)
        .await
        .map_err(|e| {
            // 内部记录详细错误，向客户端返回通用错误
            let error_id = generate_error_id();
            tracing::error!(target: "rpc", error_id = error_id, error = %e, "Failed to query static avg in postgres");
            NodegetError::DatabaseError(format!("Database error occurred. Reference: {error_id}"))
        })?;

    let json = row.map_or(Value::Array(Vec::new()), |r| r.data);
    let json = serde_json::to_string(&json)
        .map_err(|e| NodegetError::SerializationError(format!("Serialization failed: {e}")))?;

    RawValue::from_string(json).map_err(|e| {
        NodegetError::SerializationError(format!("RawValue creation error: {e}")).into()
    })
}

fn build_postgres_static_avg_sql(fields: &[StaticDataQueryField]) -> String {
    let select_columns = fields.iter().fold(String::new(), |mut output, field| {
        write!(output, ", {}", field.column_name()).expect("writing to String should not fail");
        output
    });

    let aggregate_columns = fields
        .iter()
        .copied()
        .map(build_postgres_static_field_aggregate_sql)
        .collect::<Vec<_>>()
        .join(",\n            ");

    let final_json_fields = fields.iter().fold(String::new(), |mut output, field| {
        write!(output, ", '{}', agg.{}", field.json_key(), field.json_key())
            .expect("writing to String should not fail");
        output
    });

    let aggregate_columns = if aggregate_columns.is_empty() {
        String::new()
    } else {
        format!(",\n            {aggregate_columns}")
    };

    format!(
        r"
WITH filtered AS MATERIALIZED (
    SELECT 
        timestamp{select_columns},
        MIN(timestamp) OVER () AS min_ts,
        MAX(timestamp) OVER () AS max_ts
    FROM static_monitoring
    WHERE uuid = CAST($1 AS uuid)
      AND ($2::bigint IS NULL OR timestamp >= $2)
      AND ($3::bigint IS NULL OR timestamp <= $3)
),
bucketed AS (
    SELECT
        CASE
            WHEN min_ts IS NULL THEN NULL
            WHEN min_ts = max_ts OR $4::bigint <= 1 THEN 0
            ELSE LEAST(
                $4::bigint - 1,
                ((timestamp - min_ts) * $4::bigint) / NULLIF(max_ts - min_ts, 0)
            )
        END AS bucket,
        timestamp{select_columns}
    FROM filtered
),
agg AS (
    SELECT
        bucket AS bucket,
        AVG(timestamp)::bigint AS timestamp{aggregate_columns}
    FROM bucketed
    WHERE bucket IS NOT NULL
    GROUP BY bucket
    ORDER BY bucket
)
SELECT COALESCE(
    jsonb_agg(
        jsonb_build_object(
            'uuid', $1::text,
            'timestamp', agg.timestamp{final_json_fields}
        )
        ORDER BY agg.bucket
    ),
    '[]'::jsonb
) AS data
FROM agg
"
    )
}

fn build_postgres_static_field_aggregate_sql(field: StaticDataQueryField) -> String {
    match field {
        StaticDataQueryField::Cpu => r"
jsonb_build_object(
    'physical_cores', AVG(NULLIF(bucketed.cpu_data->>'physical_cores', '')::numeric),
    'logical_cores', AVG(NULLIF(bucketed.cpu_data->>'logical_cores', '')::numeric),
    'per_core',
    (
        SELECT COALESCE(jsonb_agg(per_core.obj ORDER BY per_core.idx), '[]'::jsonb)
        FROM (
            SELECT
                arr.ord AS idx,
                jsonb_build_object(
                    'id', AVG(NULLIF(arr.elem->>'id', '')::numeric),
                    'name', NULL,
                    'vendor_id', NULL,
                    'brand', NULL
                ) AS obj
            FROM bucketed AS b2
            CROSS JOIN LATERAL jsonb_array_elements(COALESCE(b2.cpu_data->'per_core', '[]'::jsonb)) WITH ORDINALITY AS arr(elem, ord)
            WHERE b2.bucket = bucketed.bucket
            GROUP BY arr.ord
        ) AS per_core
    )
) AS cpu"
            .to_owned(),
        StaticDataQueryField::System => r"
jsonb_build_object(
    'system_name', NULL,
    'system_kernel', NULL,
    'system_kernel_version', NULL,
    'system_os_version', NULL,
    'system_os_long_version', NULL,
    'distribution_id', NULL,
    'system_host_name', NULL,
    'arch', NULL,
    'virtualization', NULL
) AS system"
            .to_owned(),
        StaticDataQueryField::Gpu => r"
(
    SELECT COALESCE(jsonb_agg(gpus.obj ORDER BY gpus.idx), '[]'::jsonb)
    FROM (
        SELECT
            arr.ord AS idx,
            jsonb_build_object(
                'id', AVG(NULLIF(arr.elem->>'id', '')::numeric),
                'name', NULL,
                'cuda_cores', AVG(NULLIF(arr.elem->>'cuda_cores', '')::numeric),
                'architecture', NULL
            ) AS obj
        FROM bucketed AS b2
        CROSS JOIN LATERAL jsonb_array_elements(COALESCE(b2.gpu_data, '[]'::jsonb)) WITH ORDINALITY AS arr(elem, ord)
        WHERE b2.bucket = bucketed.bucket
        GROUP BY arr.ord
    ) AS gpus
) AS gpu"
            .to_owned(),
    }
}
