use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

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

impl AgentConfig {
    pub async fn get_and_parse_config(
        path: impl AsRef<Path>,
    ) -> Result<AgentConfig, Box<dyn std::error::Error>> {
        let file = fs::read_to_string(path).await?;

        let config: AgentConfig = toml::from_str(&file)?;

        Ok(config)
    }
}
