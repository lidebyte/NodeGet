use crate::rpc::RpcHelper;
use crate::rpc::{rpc_exec, token_identity};
use jsonrpsee::core::{RpcResult, async_trait};
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::monitoring::data_structure::{DynamicMonitoringData, StaticMonitoringData};
use nodeget_lib::monitoring::query::{
    DynamicDataAvgQuery, DynamicDataQuery, DynamicDataQueryField, QueryCondition,
    StaticDataAvgQuery, StaticDataQuery, StaticDataQueryField,
};
use serde_json::value::RawValue;
use tracing::Instrument;
use uuid::Uuid;

mod delete_dynamic;
mod delete_static;
mod query_dynamic;
mod query_dynamic_avg;
mod query_dynamic_multi_last;
mod query_static;
mod query_static_avg;
mod query_static_multi_last;
mod report_dynamic;
mod report_static;

#[rpc(server, namespace = "agent")]
pub trait Rpc {
    #[method(name = "report_static")]
    async fn report_static(
        &self,
        token: String,
        static_monitoring_data: StaticMonitoringData,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "report_dynamic")]
    async fn report_dynamic(
        &self,
        token: String,
        dynamic_monitoring_data: DynamicMonitoringData,
    ) -> RpcResult<Box<RawValue>>;

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

    #[method(name = "query_static_avg")]
    async fn query_static_avg(
        &self,
        token: String,
        static_data_avg_query: StaticDataAvgQuery,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "query_dynamic_avg")]
    async fn query_dynamic_avg(
        &self,
        token: String,
        dynamic_data_avg_query: DynamicDataAvgQuery,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "static_data_multi_last_query")]
    async fn static_data_multi_last_query(
        &self,
        token: String,
        uuids: Vec<Uuid>,
        fields: Vec<StaticDataQueryField>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "dynamic_data_multi_last_query")]
    async fn dynamic_data_multi_last_query(
        &self,
        token: String,
        uuids: Vec<Uuid>,
        fields: Vec<DynamicDataQueryField>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "delete_static")]
    async fn delete_static(
        &self,
        token: String,
        conditions: Vec<QueryCondition>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "delete_dynamic")]
    async fn delete_dynamic(
        &self,
        token: String,
        conditions: Vec<QueryCondition>,
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
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::report_static", token_key = tk, username = un, uuid = %static_monitoring_data.uuid);
        async { rpc_exec!(report_static::report_static(token, static_monitoring_data).await) }.instrument(span).await
    }

    async fn report_dynamic(
        &self,
        token: String,
        dynamic_monitoring_data: DynamicMonitoringData,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::report_dynamic", token_key = tk, username = un, uuid = %dynamic_monitoring_data.uuid);
        async { rpc_exec!(report_dynamic::report_dynamic(token, dynamic_monitoring_data).await) }.instrument(span).await
    }

    async fn query_static(
        &self,
        token: String,
        static_data_query: StaticDataQuery,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::query_static", token_key = tk, username = un, query = ?static_data_query);
        async { rpc_exec!(query_static::query_static(token, static_data_query).await) }.instrument(span).await
    }

    async fn query_dynamic(
        &self,
        token: String,
        dynamic_data_query: DynamicDataQuery,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::query_dynamic", token_key = tk, username = un, query = ?dynamic_data_query);
        async { rpc_exec!(query_dynamic::query_dynamic(token, dynamic_data_query).await) }.instrument(span).await
    }

    async fn query_static_avg(
        &self,
        token: String,
        static_data_avg_query: StaticDataAvgQuery,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::query_static_avg", token_key = tk, username = un, query = ?static_data_avg_query);
        async { rpc_exec!(query_static_avg::query_static_avg(token, static_data_avg_query).await) }.instrument(span).await
    }

    async fn query_dynamic_avg(
        &self,
        token: String,
        dynamic_data_avg_query: DynamicDataAvgQuery,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::query_dynamic_avg", token_key = tk, username = un, query = ?dynamic_data_avg_query);
        async { rpc_exec!(query_dynamic_avg::query_dynamic_avg(token, dynamic_data_avg_query).await) }.instrument(span).await
    }

    async fn static_data_multi_last_query(
        &self,
        token: String,
        uuids: Vec<Uuid>,
        fields: Vec<StaticDataQueryField>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::static_data_multi_last_query", token_key = tk, username = un, uuids = ?uuids, fields = ?fields);
        async { rpc_exec!(query_static_multi_last::static_data_multi_last_query(token, uuids, fields).await) }.instrument(span).await
    }

    async fn dynamic_data_multi_last_query(
        &self,
        token: String,
        uuids: Vec<Uuid>,
        fields: Vec<DynamicDataQueryField>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::dynamic_data_multi_last_query", token_key = tk, username = un, uuids = ?uuids, fields = ?fields);
        async { rpc_exec!(query_dynamic_multi_last::dynamic_data_multi_last_query(token, uuids, fields).await) }.instrument(span).await
    }

    async fn delete_static(
        &self,
        token: String,
        conditions: Vec<QueryCondition>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::delete_static", token_key = tk, username = un, conditions = ?conditions);
        async { rpc_exec!(delete_static::delete_static(token, conditions).await) }.instrument(span).await
    }

    async fn delete_dynamic(
        &self,
        token: String,
        conditions: Vec<QueryCondition>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "agent::delete_dynamic", token_key = tk, username = un, conditions = ?conditions);
        async { rpc_exec!(delete_dynamic::delete_dynamic(token, conditions).await) }.instrument(span).await
    }
}
