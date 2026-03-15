use super::{default_stack, ConfigLoader, DaemonConfig, StackConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const NAME: &str = "mapky.watcher";
pub const DEFAULT_HOMESERVER: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
pub const DEFAULT_EVENTS_LIMIT: u32 = 1_000;
pub const DEFAULT_WATCHER_SLEEP: u64 = 5_000;
pub const TESTNET: bool = false;
pub const DEFAULT_TESTNET_HOST: &str = "localhost";

fn default_testnet() -> bool {
    TESTNET
}

fn default_testnet_host() -> String {
    DEFAULT_TESTNET_HOST.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub name: String,
    /// Seed/default homeserver. Additional homeservers are discovered via the ingest endpoint
    /// and persisted in Neo4j; the watcher polls all known homeservers each tick.
    pub homeserver: String,
    pub events_limit: u32,
    pub watcher_sleep: u64,
    #[serde(default = "default_testnet")]
    pub testnet: bool,
    #[serde(default = "default_testnet_host")]
    pub testnet_host: String,
    #[serde(default = "default_stack")]
    pub stack: StackConfig,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            name: NAME.to_string(),
            homeserver: DEFAULT_HOMESERVER.to_string(),
            events_limit: DEFAULT_EVENTS_LIMIT,
            watcher_sleep: DEFAULT_WATCHER_SLEEP,
            testnet: TESTNET,
            testnet_host: DEFAULT_TESTNET_HOST.to_string(),
            stack: StackConfig::default(),
        }
    }
}

impl From<DaemonConfig> for WatcherConfig {
    fn from(daemon_config: DaemonConfig) -> Self {
        WatcherConfig {
            stack: daemon_config.stack,
            ..daemon_config.watcher
        }
    }
}

impl ConfigLoader<WatcherConfig> for WatcherConfig {}
