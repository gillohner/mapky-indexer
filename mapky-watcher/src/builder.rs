use mapky_common::db::PubkyConnector;
use mapky_common::types::DynError;
use mapky_common::utils::create_shutdown_rx;
use mapky_common::{StackConfig, StackManager, WatcherConfig};
use tokio::sync::watch::Receiver;

use crate::service::MapkyWatcher;

#[derive(Debug, Default)]
pub struct MapkyWatcherBuilder(pub WatcherConfig);

impl MapkyWatcherBuilder {
    pub fn with_stack(mut config: WatcherConfig, stack: &StackConfig) -> Self {
        config.stack = stack.clone();
        Self(config)
    }

    pub async fn init_stack(&self) -> Result<(), DynError> {
        StackManager::setup(&self.0.name, &self.0.stack).await?;
        let testnet_host = if self.0.testnet {
            Some(self.0.testnet_host.as_str())
        } else {
            None
        };
        let _ = PubkyConnector::initialise(testnet_host).await;
        Ok(())
    }

    pub async fn start(self, shutdown_rx: Option<Receiver<bool>>) -> Result<(), DynError> {
        let shutdown_rx = shutdown_rx.unwrap_or_else(create_shutdown_rx);
        self.init_stack().await?;
        MapkyWatcher::start(shutdown_rx, self.0).await
    }
}
