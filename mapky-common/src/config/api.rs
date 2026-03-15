use super::{default_stack, ConfigLoader, DaemonConfig, StackConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const NAME: &str = "mapky.api";
pub const DEFAULT_PORT: u16 = 8090;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub name: String,
    pub port: u16,
    #[serde(default = "default_stack")]
    pub stack: StackConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            name: String::from(NAME),
            port: DEFAULT_PORT,
            stack: StackConfig::default(),
        }
    }
}

impl From<DaemonConfig> for ApiConfig {
    fn from(daemon_config: DaemonConfig) -> Self {
        ApiConfig {
            stack: daemon_config.stack,
            ..daemon_config.api
        }
    }
}

impl ConfigLoader<ApiConfig> for ApiConfig {}
