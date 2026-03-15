use mapky_common::types::DynError;
use mapky_common::WatcherConfig;
use tokio::sync::watch::Receiver;
use tokio::time::Duration;
use tracing::{debug, error, info};

pub struct MapkyWatcher;

impl MapkyWatcher {
    pub async fn start(
        mut shutdown_rx: Receiver<bool>,
        config: WatcherConfig,
    ) -> Result<(), DynError> {
        debug!(?config, "Running MapkyWatcher");

        let mut interval = tokio::time::interval(Duration::from_millis(config.watcher_sleep));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("SIGINT received, exiting MapKy Watcher loop");
                    break;
                }
                _ = interval.tick() => {
                    debug!("Polling homeserver for mapky.app events...");
                    // TODO: Fetch events from homeserver and dispatch to handlers
                    if let Err(e) = Self::poll_events(&config).await {
                        error!("Error polling events: {e}");
                    }
                }
            }
        }
        info!("MapKy Watcher shut down gracefully");
        Ok(())
    }

    async fn poll_events(_config: &WatcherConfig) -> Result<(), DynError> {
        // TODO: Implement homeserver event polling
        // 1. Read cursor from watcher_cursors table
        // 2. GET homeserver /events?cursor=X&limit=N
        // 3. For each event, dispatch to handle_put_event / handle_del_event
        // 4. Update cursor
        Ok(())
    }
}
