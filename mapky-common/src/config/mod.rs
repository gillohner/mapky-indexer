use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const LOG_LEVEL: Level = Level::Info;
pub const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org";

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
pub struct GeocodingConfig {
    #[serde(default = "default_nominatim_url")]
    pub nominatim_url: String,
}

fn default_nominatim_url() -> String {
    NOMINATIM_URL.to_string()
}

impl Default for GeocodingConfig {
    fn default() -> Self {
        Self {
            nominatim_url: default_nominatim_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackConfig {
    pub log_level: Level,
    pub db: DatabaseConfig,
    #[serde(default)]
    pub geocoding: GeocodingConfig,
}

impl Default for StackConfig {
    fn default() -> Self {
        Self {
            log_level: LOG_LEVEL,
            db: DatabaseConfig::default(),
            geocoding: GeocodingConfig::default(),
        }
    }
}

pub fn default_stack() -> StackConfig {
    StackConfig::default()
}
