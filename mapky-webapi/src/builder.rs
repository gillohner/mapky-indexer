use std::net::SocketAddr;
use std::path::PathBuf;

use mapky_common::db::PubkyConnector;
use mapky_common::types::DynError;
use mapky_common::utils::create_shutdown_rx;
use mapky_common::{ApiConfig, DaemonConfig, StackManager};
use tokio::sync::watch::Receiver;
use tracing::{error, info};

use crate::routes;

#[derive(Debug)]
pub struct MapkyApiBuilder(pub ApiConfig);

impl MapkyApiBuilder {
    pub async fn init_stack(&self) -> Result<(), DynError> {
        StackManager::setup(&self.0.name, &self.0.stack).await?;

        // Initialise PubkyConnector so the ingest endpoint can resolve homeservers.
        let testnet_host = if self.0.testnet {
            Some(self.0.testnet_host.as_str())
        } else {
            None
        };
        let _ = PubkyConnector::initialise(testnet_host).await;

        Ok(())
    }

    pub async fn start(self, shutdown_rx: Option<Receiver<bool>>) -> Result<MapkyApi, DynError> {
        let mut shutdown_rx = shutdown_rx.unwrap_or_else(create_shutdown_rx);

        self.init_stack()
            .await
            .inspect_err(|e| error!("Failed to initialize stack: {e}"))?;

        let api = MapkyApi::start(self.0).await?;

        info!("MapKy API listening on http://{}", api.addr);

        let _ = shutdown_rx.changed().await;
        info!("Received shutdown signal");

        Ok(api)
    }
}

pub struct MapkyApi {
    pub addr: SocketAddr,
}

impl MapkyApi {
    pub async fn start_from_daemon(
        config_dir: PathBuf,
        shutdown_rx: Option<Receiver<bool>>,
    ) -> Result<Self, DynError> {
        let daemon_config = DaemonConfig::read_or_create_config_file(config_dir).await?;
        let api_config = ApiConfig::from(daemon_config);
        MapkyApiBuilder(api_config).start(shutdown_rx).await
    }

    pub async fn start(config: ApiConfig) -> Result<Self, DynError> {
        let router = routes::routes();

        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .inspect_err(|e| error!("Failed to bind to {addr}: {e}"))?;

        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                error!("MapKy API server error: {e}");
            }
        });

        Ok(MapkyApi { addr: local_addr })
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}
