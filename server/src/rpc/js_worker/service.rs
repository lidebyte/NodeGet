use crate::entity::{js_result, js_worker};
use crate::js_runtime::runtime_pool;
use crate::js_runtime::js_runner_source_mode;
use crate::rpc::RpcHelper;
use crate::rpc::js_worker::JsWorkerRpcImpl;
use log::error;
use nodeget_lib::error::NodegetError;
use nodeget_lib::js_runtime::{JsCodeInput, RunType};
use nodeget_lib::utils::get_local_timestamp_ms_i64;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::time::Duration;

pub async fn enqueue_defined_js_worker_run(
    js_script_name: String,
    run_type: RunType,
    params: Value,
    env_override: Option<Value>,
) -> anyhow::Result<i64> {
    let script_name = js_script_name.trim().to_owned();
    if script_name.is_empty() {
        return Err(NodegetError::InvalidInput("js_script_name cannot be empty".to_owned()).into());
    }

    let db = JsWorkerRpcImpl::get_db()?.clone();
    let model = js_worker::Entity::find()
        .filter(js_worker::Column::Name.eq(script_name.as_str()))
        .one(&db)
        .await
        .map_err(|e| NodegetError::DatabaseError(e.to_string()))?
        .ok_or_else(|| NodegetError::NotFound(format!("js_worker not found: {script_name}")))?;

    let worker_id = model.id;
    let worker_name = model.name.clone();
    let bytecode = model.js_byte_code.ok_or_else(|| {
        NodegetError::InvalidInput(format!(
            "js_worker '{script_name}' has no precompiled bytecode"
        ))
    })?;
    let runtime_clean_time = model.runtime_clean_time;
    let resolved_env =
        env_override.unwrap_or_else(|| model.env.unwrap_or_else(|| serde_json::json!({})));
    let run_type_text = run_type.as_str().to_owned();

    let start_time = get_local_timestamp_ms_i64().unwrap_or(0);
    let insert_result = js_result::Entity::insert(js_result::ActiveModel {
        id: ActiveValue::NotSet,
        js_worker_id: Set(worker_id),
        js_worker_name: Set(worker_name.clone()),
        run_type: Set(run_type_text),
        start_time: Set(Some(start_time)),
        finish_time: Set(None),
        param: Set(Some(params.clone())),
        result: Set(None),
        error_message: Set(None),
    })
    .exec(&db)
    .await
    .map_err(|e| NodegetError::DatabaseError(e.to_string()))?;

    let js_result_id = insert_result.last_insert_id;

    tokio::spawn(async move {
        let run_outcome = runtime_pool::init_global_pool()
            .execute_script(
                worker_name.as_str(),
                bytecode,
                run_type,
                params,
                resolved_env,
                runtime_clean_time,
            )
            .await;

        let finish_time = get_local_timestamp_ms_i64().unwrap_or(start_time);
        let (result_json, mut error_message): (Option<Value>, Option<String>) = match run_outcome {
            Ok(value) => (Some(value), None),
            Err(e) => (
                None,
                Some(format!("JavaScript runtime execution failed: {e}")),
            ),
        };

        if result_json.is_none() && error_message.is_none() {
            error_message = Some("JavaScript run finished without result or error".to_owned());
        }

        if let Err(e) = js_result::Entity::update_many()
            .set(js_result::ActiveModel {
                finish_time: Set(Some(finish_time)),
                result: Set(result_json),
                error_message: Set(error_message),
                ..Default::default()
            })
            .filter(js_result::Column::Id.eq(js_result_id))
            .exec(&db)
            .await
        {
            error!("Failed to update js_result {js_result_id} for worker '{worker_name}': {e}");
        }
    });

    Ok(js_result_id)
}

pub async fn run_inline_call_and_record_result(
    js_script_name: String,
    params: Value,
    timeout_sec: Option<f64>,
    inline_caller: Option<String>,
) -> anyhow::Result<Value> {
    let script_name = js_script_name.trim().to_owned();
    if script_name.is_empty() {
        return Err(NodegetError::InvalidInput("js_worker_name cannot be empty".to_owned()).into());
    }

    let timeout_duration = match timeout_sec {
        Some(value) if value.is_finite() && value > 0.0 => Some(Duration::from_secs_f64(value)),
        Some(value) => {
            return Err(NodegetError::InvalidInput(format!(
                "timeout_sec must be a positive finite number, got: {value}"
            ))
            .into());
        }
        None => None,
    };

    let db = JsWorkerRpcImpl::get_db()?.clone();
    let model = js_worker::Entity::find()
        .filter(js_worker::Column::Name.eq(script_name.as_str()))
        .one(&db)
        .await
        .map_err(|e| NodegetError::DatabaseError(e.to_string()))?
        .ok_or_else(|| NodegetError::NotFound(format!("js_worker not found: {script_name}")))?;

    let worker_id = model.id;
    let worker_name = model.name.clone();
    let bytecode = model.js_byte_code.ok_or_else(|| {
        NodegetError::InvalidInput(format!(
            "js_worker '{script_name}' has no precompiled bytecode"
        ))
    })?;
    let env = model.env.unwrap_or_else(|| serde_json::json!({}));

    let start_time = get_local_timestamp_ms_i64().unwrap_or(0);
    let insert_result = js_result::Entity::insert(js_result::ActiveModel {
        id: ActiveValue::NotSet,
        js_worker_id: Set(worker_id),
        js_worker_name: Set(worker_name.clone()),
        run_type: Set(RunType::InlineCall.as_str().to_owned()),
        start_time: Set(Some(start_time)),
        finish_time: Set(None),
        param: Set(Some(params.clone())),
        result: Set(None),
        error_message: Set(None),
    })
    .exec(&db)
    .await
    .map_err(|e| NodegetError::DatabaseError(e.to_string()))?;
    let js_result_id = insert_result.last_insert_id;

    let target_script_name = worker_name.clone();
    let run_task = tokio::task::spawn_blocking(move || {
        crate::js_runtime::js_runner(
            JsCodeInput::Bytecode(bytecode),
            RunType::InlineCall,
            params,
            env,
            Some(target_script_name),
            inline_caller,
        )
        .map_err(|e| {
            NodegetError::Other(format!("JavaScript runtime execution failed: {e}")).into()
        })
    });

    let run_outcome: anyhow::Result<Value> = if let Some(duration) = timeout_duration {
        match tokio::time::timeout(duration, run_task).await {
            Ok(join_result) => join_result.map_err(|e| {
                anyhow::Error::from(NodegetError::Other(format!(
                    "inline_call task join failed: {e}"
                )))
            })?,
            Err(_) => Err(NodegetError::Other(format!(
                "inline_call timed out after {:.3} seconds",
                duration.as_secs_f64()
            ))
            .into()),
        }
    } else {
        run_task.await.map_err(|e| {
            anyhow::Error::from(NodegetError::Other(format!(
                "inline_call task join failed: {e}"
            )))
        })?
    };

    let finish_time = get_local_timestamp_ms_i64().unwrap_or(start_time);
    let (result_json, mut error_message, return_value): (
        Option<Value>,
        Option<String>,
        anyhow::Result<Value>,
    ) = match run_outcome {
        Ok(value) => (Some(value.clone()), None, Ok(value)),
        Err(e) => (None, Some(e.to_string()), Err(e)),
    };

    if result_json.is_none() && error_message.is_none() {
        error_message = Some("JavaScript inline_call finished without result or error".to_owned());
    }

    if let Err(e) = js_result::Entity::update_many()
        .set(js_result::ActiveModel {
            finish_time: Set(Some(finish_time)),
            result: Set(result_json),
            error_message: Set(error_message),
            ..Default::default()
        })
        .filter(js_result::Column::Id.eq(js_result_id))
        .exec(&db)
        .await
    {
        error!(
            "Failed to update js_result {js_result_id} for inline_call worker '{worker_name}': {e}"
        );
    }

    return_value
}

pub async fn enqueue_source_js_worker_run(
    js_script_name: String,
    run_type: RunType,
    params: Value,
    env_override: Option<Value>,
) -> anyhow::Result<i64> {
    let script_name = js_script_name.trim().to_owned();
    if script_name.is_empty() {
        return Err(NodegetError::InvalidInput("js_script_name cannot be empty".to_owned()).into());
    }

    let db = JsWorkerRpcImpl::get_db()?.clone();
    let model = js_worker::Entity::find()
        .filter(js_worker::Column::Name.eq(script_name.as_str()))
        .one(&db)
        .await
        .map_err(|e| NodegetError::DatabaseError(e.to_string()))?
        .ok_or_else(|| NodegetError::NotFound(format!("js_worker not found: {script_name}")))?;

    let worker_id = model.id;
    let worker_name = model.name.clone();
    let source_code = model.js_script;
    let resolved_env =
        env_override.unwrap_or_else(|| model.env.unwrap_or_else(|| serde_json::json!({})));
    let run_type_text = run_type.as_str().to_owned();

    let start_time = get_local_timestamp_ms_i64().unwrap_or(0);
    let insert_result = js_result::Entity::insert(js_result::ActiveModel {
        id: ActiveValue::NotSet,
        js_worker_id: Set(worker_id),
        js_worker_name: Set(worker_name.clone()),
        run_type: Set(run_type_text),
        start_time: Set(Some(start_time)),
        finish_time: Set(None),
        param: Set(Some(params.clone())),
        result: Set(None),
        error_message: Set(None),
    })
    .exec(&db)
    .await
    .map_err(|e| NodegetError::DatabaseError(e.to_string()))?;

    let js_result_id = insert_result.last_insert_id;

    let worker_name_for_log = worker_name.clone();
    tokio::spawn(async move {
        let worker_name_in_spawn = worker_name_for_log.clone();
        let run_outcome: Result<Value, String> = match tokio::task::spawn_blocking(move || {
            js_runner_source_mode(
                &source_code,
                &worker_name,
                run_type,
                params,
                resolved_env,
            )
        })
        .await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(format!("JavaScript execution error: {e}")),
            Err(e) => Err(format!("Source mode task join failed: {e}")),
        };

        let finish_time = get_local_timestamp_ms_i64().unwrap_or(start_time);
        let (result_json, mut error_message): (Option<Value>, Option<String>) = match run_outcome {
            Ok(value) => (Some(value), None),
            Err(e) => (
                None,
                Some(format!("JavaScript runtime execution failed: {e}")),
            ),
        };

        if result_json.is_none() && error_message.is_none() {
            error_message = Some("JavaScript run finished without result or error".to_owned());
        }

        if let Err(e) = js_result::Entity::update_many()
            .set(js_result::ActiveModel {
                finish_time: Set(Some(finish_time)),
                result: Set(result_json),
                error_message: Set(error_message),
                ..Default::default()
            })
            .filter(js_result::Column::Id.eq(js_result_id))
            .exec(&db)
            .await
        {
            error!("Failed to update js_result {js_result_id} for source mode worker '{worker_name_in_spawn}': {e}");
        }
    });

    Ok(js_result_id)
}
