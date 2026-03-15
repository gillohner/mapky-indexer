use super::{default_stack, ConfigLoader, DaemonConfig, StackConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const NAME: &str = "mapky.api";
pub const DEFAULT_PORT: u16 = 8090;
pub const TESTNET: bool = false;
pub const DEFAULT_TESTNET_HOST: &str = "localhost";

fn default_testnet() -> bool {
    TESTNET
}

fn default_testnet_host() -> String {
    DEFAULT_TESTNET_HOST.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub name: String,
    pub port: u16,
    /// Whether to use the Pubky testnet (needed for the ingest endpoint).
    #[serde(default = "default_testnet")]
    pub testnet: bool,
    /// Testnet host address (only used when `testnet = true`).
    #[serde(default = "default_testnet_host")]
    pub testnet_host: String,
    #[serde(default = "default_stack")]
    pub stack: StackConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            name: String::from(NAME),
            port: DEFAULT_PORT,
            testnet: TESTNET,
            testnet_host: DEFAULT_TESTNET_HOST.to_string(),
            stack: StackConfig::default(),
        }
    }
}

impl From<DaemonConfig> for ApiConfig {
    fn from(daemon_config: DaemonConfig) -> Self {
        ApiConfig {
            // Inherit testnet settings from the watcher config so they stay in sync.
            testnet: daemon_config.watcher.testnet,
            testnet_host: daemon_config.watcher.testnet_host.clone(),
            stack: daemon_config.stack,
            ..daemon_config.api
        }
    }
}

impl ConfigLoader<ApiConfig> for ApiConfig {}
