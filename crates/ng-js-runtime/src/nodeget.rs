//! `nodeget()` API —— JS 上下文内部发起 JSON-RPC 调用。
//!
//! JS 端通过 `globalThis.__nodeget_rpc_raw(json)` 触发此模块，
//! 将请求转发到服务器 `RpcModule` 进行分发。支持单条和批量请求。

use crate::js_worker_service::get_js_worker_service;
use crate::server_runtime::js_error;
use crate::spawn_on_server_runtime::spawn_on_server_runtime;
use futures_util::future::join_all;
use rquickjs::Error;
use serde_json::value::RawValue;
use std::result::Result as StdResult;
use tracing::{debug, trace};

/// 处理单条 JSON-RPC 请求。
///
/// 将请求字符串转发到 `RawJsonDispatcher::raw_json_request`，
/// 通过 `spawn_on_server_runtime` 在服务器 Runtime 上执行。
async fn raw_single_request(json: &str) -> StdResult<String, Error> {
    trace!(target: "js_runtime", "processing raw JSON-RPC request from JS");
    let json = json.to_owned();

    let response = spawn_on_server_runtime(async move {
        let rpc_module = get_js_worker_service().get_rpc_module().await;
        let (resp, _stream) = rpc_module
            .raw_json_request(&json, 16)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(resp)
    })
    .await
    .map_err(|e| js_error("jsonrpc_module", e))?;

    response.map_err(|e| js_error("jsonrpc_module", e))
}

/// 从 JS 上下文发起 `nodeget()` RPC 调用，返回响应 JSON 字符串。
///
/// - `json` —— JSON-RPC 请求字符串（单条或批量数组）
///
/// 内部步骤：
/// 1. 判断是否为批量请求（以 `[` 开头）
/// 2. 批量请求：用 `RawValue` 避免解析-序列化-再解析的开销，并行执行所有子请求
/// 3. 单条请求：直接转发到 `raw_single_request`
///
/// # Errors
/// 若 JSON-RPC 请求执行失败，返回 `rquickjs::Error`。
pub async fn js_nodeget(json: String) -> StdResult<String, Error> {
    debug!(target: "js_runtime", "handling JS nodeget RPC call");
    let trimmed = json.trim();

    // 批量请求：JSON 数组形式，并行处理每条子请求
    if trimmed.starts_with('[') {
        // 使用 RawValue 避免 parse→serialize→parse 的往返开销
        let items: Vec<Box<RawValue>> =
            serde_json::from_str(trimmed).map_err(|e| js_error("jsonrpc_parse", e.to_string()))?;

        let futs: Vec<_> = items
            .iter()
            .map(|item| {
                let req_str = item.get();
                async move { raw_single_request(req_str).await }
            })
            .collect();

        let results = join_all(futs).await;

        let mut responses = Vec::with_capacity(results.len());
        for result in results {
            responses.push(result?);
        }

        Ok(format!("[{}]", responses.join(",")))
    } else {
        // 单条请求
        raw_single_request(trimmed).await
    }
}
