use std::path::PathBuf;

use mapky_common::types::DynError;
use mapky_common::utils::create_shutdown_rx;
use mapky_common::DaemonConfig;
use mapky_watcher::MapkyWatcherBuilder;
use mapky_webapi::MapkyApiBuilder;
use mapky_common::ApiConfig;
use mapky_common::WatcherConfig;
use serde::{Deserialize, Serialize};
use tokio::{sync::watch::Receiver, try_join};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonLauncher;

impl DaemonLauncher {
    /// Starts both the API and Watcher services concurrently.
    ///
    /// Blocks until one service errors or the shutdown signal is received.
    pub async fn start(
        config_dir: PathBuf,
        shutdown_rx: Option<Receiver<bool>>,
    ) -> Result<(), DynError> {
        let shutdown_rx = shutdown_rx.unwrap_or_else(create_shutdown_rx);

        let config = DaemonConfig::read_or_create_config_file(config_dir).await?;

        let api_config = ApiConfig::from(config.clone());
        let api_builder = MapkyApiBuilder(api_config);

        let watcher_config = WatcherConfig::from(config.clone());
        let watcher_builder = MapkyWatcherBuilder::with_stack(watcher_config, &config.stack);

        try_join!(
            api_builder.start(Some(shutdown_rx.clone())),
            watcher_builder.start(Some(shutdown_rx))
        )?;

        Ok(())
    }
}
