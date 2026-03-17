use super::{ApiConfig, ConfigLoader, StackConfig, WatcherConfig};
use crate::types::DynError;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use tracing::error;

pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub watcher: WatcherConfig,
    pub stack: StackConfig,
}

impl DaemonConfig {
    pub async fn read_or_create_config_file(
        config_dir: PathBuf,
    ) -> Result<DaemonConfig, DynError> {
        let config_file_path = config_dir.join(CONFIG_FILE_NAME);

        if !config_file_path.exists() {
            if let Some(parent) = config_file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let default_toml = include_str!("../../../config/config.example.toml");
            std::fs::write(&config_file_path, default_toml)?;
        }

        println!("mapkyd loading config file {}", config_file_path.display());
        Self::load(&config_file_path).await.inspect_err(|e| {
            error!("Failed to load config file: {e}");
        })
    }
}

impl ConfigLoader<DaemonConfig> for DaemonConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toml_parsing() {
        let config = DaemonConfig::read_or_create_config_file(
            tempfile::TempDir::new().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap();

        assert_eq!(config.api.name, "mapkyd.api");
        assert_eq!(config.api.port, 8090);
        assert_eq!(config.watcher.name, "mapkyd.watcher");
        assert_eq!(config.watcher.events_limit, 1000);
        assert_eq!(config.watcher.watcher_sleep, 5000);
        assert!(!config.watcher.testnet);
        assert_eq!(config.watcher.testnet_host, "localhost");
        assert_eq!(config.stack.db.neo4j.uri, "bolt://localhost:7687");
        assert_eq!(
            config.stack.db.postgres.url,
            "postgres://mapky:mapky@localhost:5432/mapky"
        );
        assert_eq!(
            config.stack.geocoding.nominatim_url,
            "https://nominatim.openstreetmap.org"
        );
    }
}
