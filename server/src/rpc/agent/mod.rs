mod query;
mod report;

use crate::rpc::RpcHelper;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use serde_json::Value;

#[rpc(server, namespace = "agent")]
pub trait Rpc {
    #[method(name = "report_static")]
    async fn report_static(&self, token: String, data: Value) -> Value;

    #[method(name = "report_dynamic")]
    async fn report_dynamic(&self, token: String, data: Value) -> Value;

    #[method(name = "query_static")]
    async fn query_static(&self, token: String, data: Value) -> Value;

    #[method(name = "query_dynamic")]
    async fn query_dynamic(&self, token: String, data: Value) -> Value;
}
pub struct AgentRpcImpl;

impl RpcHelper for AgentRpcImpl {}

#[async_trait]
impl RpcServer for AgentRpcImpl {
    async fn report_static(&self, token: String, data: Value) -> Value {
        report::report_static(token, data).await
    }

    async fn report_dynamic(&self, token: String, data: Value) -> Value {
        report::report_dynamic(token, data).await
    }

    async fn query_static(&self, token: String, data: Value) -> Value {
        query::query_static(token, data).await
    }

    async fn query_dynamic(&self, token: String, data: Value) -> Value {
        query::query_dynamic(token, data).await
    }
}
