use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const LOG_LEVEL: Level = Level::Info;

mod api;
mod daemon;
mod db;
mod loader;
mod watcher;

pub use api::ApiConfig;
pub use daemon::DaemonConfig;
pub use db::{DatabaseConfig, Neo4JConfig, PostgresConfig};
pub use loader::ConfigLoader;
pub use watcher::WatcherConfig;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackConfig {
    pub log_level: Level,
    pub db: DatabaseConfig,
}

impl Default for StackConfig {
    fn default() -> Self {
        Self {
            log_level: LOG_LEVEL,
            db: DatabaseConfig::default(),
        }
    }
}

pub fn default_stack() -> StackConfig {
    StackConfig::default()
}
