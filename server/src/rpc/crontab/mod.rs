mod create;
mod delete;
mod get;
mod set_enable;
mod toggle_enable;

use crate::rpc::RpcHelper;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::crontab::CronType;
use serde_json::Value;

#[rpc(server, namespace = "crontab")]
pub trait Rpc {
    #[method(name = "create")]
    async fn create(
        &self,
        token: String,
        name: String,
        cron_expression: String,
        cron_type: CronType,
    ) -> Value;

    #[method(name = "get")]
    async fn get(&self, token: String) -> Value;

    #[method(name = "delete")]
    async fn delete(&self, token: String, name: String) -> Value;

    #[method(name = "toggle_enable")]
    async fn toggle_enable(&self, token: String, name: String) -> Value;

    #[method(name = "set_enable")]
    async fn set_enable(&self, token: String, name: String, enable: bool) -> Value;
}

pub struct CrontabRpcImpl;

impl RpcHelper for CrontabRpcImpl {}

#[async_trait]
impl RpcServer for CrontabRpcImpl {
    async fn create(
        &self,
        token: String,
        name: String,
        cron_expression: String,
        cron_type: CronType,
    ) -> Value {
        create::create(token, name, cron_expression, cron_type).await
    }

    async fn get(&self, token: String) -> Value {
        get::get(token).await
    }

    async fn delete(&self, token: String, name: String) -> Value {
        delete::delete(token, name).await
    }

    async fn toggle_enable(&self, token: String, name: String) -> Value {
        toggle_enable::toggle_enable(token, name).await
    }

    async fn set_enable(&self, token: String, name: String, enable: bool) -> Value {
        set_enable::set_enable(token, name, enable).await
    }
}
