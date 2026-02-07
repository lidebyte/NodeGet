use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use log::info;
use nodeget_lib::utils::version::NodeGetVersion;
use serde_json::Value;

#[rpc(server, namespace = "nodeget-server")]
pub trait Rpc {
    #[method(name = "hello")]
    async fn hello(&self) -> String;

    #[method(name = "version")]
    async fn version(&self) -> Value;
}

pub struct NodegetServerRpcImpl;

#[async_trait]
impl RpcServer for NodegetServerRpcImpl {
    async fn hello(&self) -> String {
        info!("Hello Request");
        "NodeGet Server Is Running!".to_string()
    }

    async fn version(&self) -> Value {
        info!("Version Request");
        serde_json::to_value(NodeGetVersion::get()).unwrap()
    }
}
