//! 内联 JS 调用：从一个 JS Worker 内部调用另一个 Worker。
//!
//! JS 端通过 `globalThis.__nodeget_inline_call_raw(name, paramsJson, timeoutSec, caller)`
//! 触发此模块，将请求转发到 `JsWorkerService::run_inline_call_and_record_result`。

use crate::js_worker_service::get_js_worker_service;
use crate::server_runtime::js_error;
use crate::spawn_on_server_runtime::spawn_on_server_runtime;
use rquickjs::Error;
use serde_json::Value;
use std::result::Result as StdResult;
use tracing::debug;

/// 从 JS 上下文发起内联调用，执行目标 Worker 并返回结果 JSON 字符串。
///
/// - `js_worker_name` —— 目标 Worker 名称
/// - `params_json` —— 调用参数的 JSON 字符串
/// - `timeout_sec` —— 调用方指定的软超时（秒），None 则不限
/// - `inline_caller` —— 发起调用的源 Worker 名称，用于审计
///
/// 内部步骤：
/// 1. 解析 `params_json` 为 `serde_json::Value`
/// 2. 通过 `spawn_on_server_runtime` 在服务器 Runtime 上执行（避免跨 Runtime 资源冲突）
/// 3. 调用 `JsWorkerService::run_inline_call_and_record_result` 并记录执行结果
/// 4. 将返回值序列化为 JSON 字符串
///
/// # Errors
/// 若参数非合法 JSON 或内联调用执行失败，返回 `rquickjs::Error`。
pub async fn js_inline_call(
    js_worker_name: String,
    params_json: String,
    timeout_sec: Option<f64>,
    inline_caller: Option<String>,
) -> StdResult<String, Error> {
    debug!(target: "js_runtime", js_worker_name = %js_worker_name, "executing inline call");
    let params: Value = serde_json::from_str(&params_json).map_err(|e| {
        js_error(
            "inline_call",
            format!("inline_call params is not valid JSON: {e}"),
        )
    })?;

    let result_json = spawn_on_server_runtime(async move {
        let result_value = get_js_worker_service()
            .run_inline_call_and_record_result(js_worker_name, params, timeout_sec, inline_caller)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result_value)
            .map_err(|e| format!("Failed to serialize inline_call result: {e}"))
    })
    .await
    .map_err(|e| js_error("inline_call", e))?
    .map_err(|e| js_error("inline_call", e))?;

    Ok(result_json)
}
