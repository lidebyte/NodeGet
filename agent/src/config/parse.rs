use crate::config::AgentConfig;
use std::path::Path;
use tokio::fs;

impl AgentConfig {
    pub async fn get_and_parse_config(
        path: impl AsRef<Path>,
    ) -> Result<AgentConfig, Box<dyn std::error::Error>> {
        let file = fs::read_to_string(path).await?;

        let config: AgentConfig = toml::from_str(&file)?;

        Ok(config)
    }
}
