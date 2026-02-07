use crate::rpc::RpcHelper;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use nodeget_lib::metadata;
use serde_json::Value;
use uuid::Uuid;

mod get;
mod write;

#[rpc(server, namespace = "metadata")]
pub trait Rpc {
    #[method(name = "get")]
    async fn get(&self, token: String, uuid: Uuid) -> Value;

    #[method(name = "write")]
    async fn write(&self, token: String, metadata: metadata::Metadata) -> Value;
}

pub struct MetadataRpcImpl;

impl RpcHelper for MetadataRpcImpl {}

#[async_trait]
impl RpcServer for MetadataRpcImpl {
    async fn get(&self, token: String, uuid: Uuid) -> Value {
        get::get(token, uuid).await
    }

    async fn write(&self, token: String, metadata: metadata::Metadata) -> Value {
        write::write(token, metadata).await
    }
}
