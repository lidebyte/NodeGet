use crate::rpc::RpcHelper;
use crate::rpc::{rpc_exec, token_identity};
use jsonrpsee::core::RpcResult;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::crontab_result::query::CrontabResultDataQuery;
use serde::Deserialize;
use serde::Serialize;
use serde_json::value::RawValue;
use tracing::Instrument;

mod auth;
mod delete;
mod query;

/// `CrontabResult` 删除参数
#[derive(Debug, Serialize, Deserialize)]
pub struct CrontabResultDelete {
    /// 可选的 `cron_name` 过滤，若指定则只删除该 `cron_name` 的记录
    pub cron_name: Option<String>,
    /// 删除该时间之前的记录（毫秒时间戳）
    pub before_time: i64,
}

#[rpc(server, namespace = "crontab-result")]
pub trait Rpc {
    #[method(name = "query")]
    async fn query(&self, token: String, query: CrontabResultDataQuery)
    -> RpcResult<Box<RawValue>>;

    #[method(name = "delete")]
    async fn delete(
        &self,
        token: String,
        delete_params: CrontabResultDelete,
    ) -> RpcResult<Box<RawValue>>;
}

pub struct CrontabResultRpcImpl;

impl RpcHelper for CrontabResultRpcImpl {}

#[async_trait]
impl RpcServer for CrontabResultRpcImpl {
    async fn query(
        &self,
        token: String,
        query: CrontabResultDataQuery,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "crontab-result::query", token_key = tk, username = un, query = ?query);
        async { rpc_exec!(query::query(token, query).await) }
            .instrument(span)
            .await
    }

    async fn delete(
        &self,
        token: String,
        delete_params: CrontabResultDelete,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "rpc", "crontab-result::delete", token_key = tk, username = un, delete_params = ?delete_params);
        async { rpc_exec!(delete::delete(token, delete_params).await) }
            .instrument(span)
            .await
    }
}
