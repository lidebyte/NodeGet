use crate::rpc::RpcHelper;
use jsonrpsee::core::{RpcResult, async_trait};
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::monitoring::data_structure::{DynamicMonitoringData, StaticMonitoringData};
use nodeget_lib::monitoring::query::{DynamicDataQuery, StaticDataQuery};
use serde_json::Value;
use serde_json::value::RawValue;

mod query_dynamic;
mod query_static;
mod report_dynamic;
mod report_static;

#[rpc(server, namespace = "agent")]
pub trait Rpc {
    #[method(name = "report_static")]
    async fn report_static(
        &self,
        token: String,
        static_monitoring_data: StaticMonitoringData,
    ) -> Value;

    #[method(name = "report_dynamic")]
    async fn report_dynamic(
        &self,
        token: String,
        dynamic_monitoring_data: DynamicMonitoringData,
    ) -> Value;

    #[method(name = "query_static")]
    async fn query_static(
        &self,
        token: String,
        static_data_query: StaticDataQuery,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "query_dynamic")]
    async fn query_dynamic(
        &self,
        token: String,
        dynamic_data_query: DynamicDataQuery,
    ) -> RpcResult<Box<RawValue>>;
}

pub struct AgentRpcImpl;

impl RpcHelper for AgentRpcImpl {}

#[async_trait]
impl RpcServer for AgentRpcImpl {
    async fn report_static(
        &self,
        token: String,
        static_monitoring_data: StaticMonitoringData,
    ) -> Value {
        report_static::report_static(token, static_monitoring_data).await
    }

    async fn report_dynamic(
        &self,
        token: String,
        dynamic_monitoring_data: DynamicMonitoringData,
    ) -> Value {
        report_dynamic::report_dynamic(token, dynamic_monitoring_data).await
    }

    async fn query_static(
        &self,
        token: String,
        static_data_query: StaticDataQuery,
    ) -> RpcResult<Box<RawValue>> {
        query_static::query_static(token, static_data_query).await
    }

    async fn query_dynamic(
        &self,
        token: String,
        dynamic_data_query: DynamicDataQuery,
    ) -> RpcResult<Box<RawValue>> {
        query_dynamic::query_dynamic(token, dynamic_data_query).await
    }
}
