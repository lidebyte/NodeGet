use crate::rpc::RpcHelper;
use crate::rpc::{rpc_exec, token_identity};
use jsonrpsee::core::RpcResult;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use serde_json::Value;
use serde_json::value::RawValue;
use tracing::Instrument;

mod auth;
mod create;
mod delete;
mod get_rt_pool;
mod list_all_js_worker;
mod read;
mod route_name;
mod run;
pub mod service;
mod update;

#[rpc(server, namespace = "js-worker")]
pub trait Rpc {
    #[method(name = "create")]
    async fn create(
        &self,
        token: String,
        name: String,
        description: Option<String>,
        js_script_base64: String,
        route_name: Option<String>,
        runtime_clean_time: Option<i64>,
        env: Option<Value>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "update")]
    async fn update(
        &self,
        token: String,
        name: String,
        description: Option<String>,
        js_script_base64: String,
        route_name: Option<String>,
        runtime_clean_time: Option<i64>,
        env: Option<Value>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "delete")]
    async fn delete(&self, token: String, name: String) -> RpcResult<Box<RawValue>>;

    #[method(name = "read")]
    async fn read(&self, token: String, name: String) -> RpcResult<Box<RawValue>>;

    #[method(name = "run")]
    async fn run(
        &self,
        token: String,
        js_script_name: String,
        run_type: Option<nodeget_lib::js_runtime::RunType>,
        params: Value,
        env: Option<Value>,
        compile_mode: Option<nodeget_lib::js_runtime::CompileMode>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "get_rt_pool")]
    async fn get_rt_pool(&self, token: String) -> RpcResult<Box<RawValue>>;

    #[method(name = "list_all_js_worker")]
    async fn list_all_js_worker(&self, token: String) -> RpcResult<Box<RawValue>>;
}

pub struct JsWorkerRpcImpl;

impl RpcHelper for JsWorkerRpcImpl {}

#[async_trait]
impl RpcServer for JsWorkerRpcImpl {
    async fn create(
        &self,
        token: String,
        name: String,
        description: Option<String>,
        js_script_base64: String,
        route_name: Option<String>,
        runtime_clean_time: Option<i64>,
        env: Option<Value>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::create", token_key = tk, username = un, name = %name, description = ?description, route_name = ?route_name, runtime_clean_time = ?runtime_clean_time);
        async {
            rpc_exec!(
                create::create(
                    token,
                    name,
                    description,
                    js_script_base64,
                    route_name,
                    runtime_clean_time,
                    env
                )
                .await
            )
        }
        .instrument(span)
        .await
    }

    async fn update(
        &self,
        token: String,
        name: String,
        description: Option<String>,
        js_script_base64: String,
        route_name: Option<String>,
        runtime_clean_time: Option<i64>,
        env: Option<Value>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::update", token_key = tk, username = un, name = %name, description = ?description, route_name = ?route_name, runtime_clean_time = ?runtime_clean_time);
        async {
            rpc_exec!(
                update::update(
                    token,
                    name,
                    description,
                    js_script_base64,
                    route_name,
                    runtime_clean_time,
                    env
                )
                .await
            )
        }
        .instrument(span)
        .await
    }

    async fn delete(&self, token: String, name: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::delete", token_key = tk, username = un, name = %name);
        async { rpc_exec!(delete::delete(token, name).await) }
            .instrument(span)
            .await
    }

    async fn read(&self, token: String, name: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::read", token_key = tk, username = un, name = %name);
        async { rpc_exec!(read::read(token, name).await) }
            .instrument(span)
            .await
    }

    async fn run(
        &self,
        token: String,
        js_script_name: String,
        run_type: Option<nodeget_lib::js_runtime::RunType>,
        params: Value,
        env: Option<Value>,
        compile_mode: Option<nodeget_lib::js_runtime::CompileMode>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::run", token_key = tk, username = un, js_script_name = %js_script_name, run_type = ?run_type, compile_mode = ?compile_mode);
        async {
            rpc_exec!(run::run(token, js_script_name, run_type, params, env, compile_mode).await)
        }
        .instrument(span)
        .await
    }

    async fn get_rt_pool(&self, token: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::get_rt_pool", token_key = tk, username = un);
        async { rpc_exec!(get_rt_pool::get_rt_pool(token).await) }
            .instrument(span)
            .await
    }

    async fn list_all_js_worker(&self, token: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "js-worker::list_all_js_worker", token_key = tk, username = un);
        async { rpc_exec!(list_all_js_worker::list_all_js_worker(token).await) }
            .instrument(span)
            .await
    }
}
