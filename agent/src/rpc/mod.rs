pub mod monitoring_data_report;
pub mod multi_server;

use std::time::Duration;
use log::{error, warn};
use nodeget_lib::task::TaskEvent;
use nodeget_lib::utils::JsonError;
use serde::{Deserialize, Serialize};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use crate::AGENT_CONFIG;
use crate::rpc::multi_server::subscribe_to;

#[derive(Serialize, Deserialize)]
struct JsonRpc {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

pub fn wrap_json_into_rpc_with_id_1(method: &str, params: Vec<serde_json::Value>) -> String {
    let rpc = JsonRpc {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: method.to_string(),
        params,
    };

    serde_json::to_string(&rpc).unwrap()
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcTask {
    pub jsonrpc: String,
    pub method: String,
    pub params: JsonRpcTaskResult,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcTaskResult {
    pub result: TaskEvent,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcErrorMessage {
    pub result: JsonError,
}


pub async fn handle_error_message() {
    time::sleep(Duration::from_secs(1)).await;

    let agent_config = AGENT_CONFIG.get().expect("Agent config not initialized");

    for server in agent_config.server.clone().unwrap_or(vec![]) {
        tokio::spawn(async move {
            let mut rx = match subscribe_to(server.name.as_str()).await {
                Ok(rx) => rx,
                Err(e) => {
                    error!("[{}] Handle Error Message Error: {}", server.name, e);
                    return;
                }
            };

            while let Ok(message) = rx.recv().await {
                let server_name = server.name.clone();
                tokio::spawn(async move {
                    let rpc = match message {
                        Message::Text(text) => text.to_string(),
                        _ => {
                            return;
                        }
                    };

                    let Ok(json) = serde_json::from_str::<JsonRpcErrorMessage>(&rpc) else {
                        return;
                    };

                    warn!(
                        "[{}] Received Error Message: {}: {}",
                        server_name, json.result.error_id, json.result.error_message
                    );
                });
            }
        });
    }
}
