mod parse;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentConfig {
    pub log_level: String,
    pub monitoring_report_interval_ms: Option<u64>,

    pub agent_uuid: String,
    pub connect_timeout_ms: Option<u64>, // ms
    pub server: Option<Vec<Server>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub name: String, // Only For Agent
    pub uuid: String,
    pub token: String,
    pub ws_url: String,

    pub allow_icmp_ping: Option<bool>,
    pub allow_tcp_ping: Option<bool>,
    pub allow_http_ping: Option<bool>,

    pub allow_ssh: Option<bool>,
    pub allow_edit_config: Option<bool>, // Dangerous
}
