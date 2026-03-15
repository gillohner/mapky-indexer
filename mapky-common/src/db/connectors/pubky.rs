use pubky::{Pubky, PubkyHttpClient};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::OnceCell;
use tracing::debug;

static PUBKY_SINGLETON: OnceCell<Arc<Pubky>> = OnceCell::const_new();

#[derive(Debug, Error)]
pub enum PubkyClientError {
    #[error("PubkyClient not initialized")]
    NotInitialized,
    #[error("Client initialization error: {0}")]
    ClientError(String),
}

pub struct PubkyConnector;

impl PubkyConnector {
    pub async fn initialise(testnet_host: Option<&str>) -> Result<(), PubkyClientError> {
        PUBKY_SINGLETON
            .get_or_try_init(|| async {
                let mode = testnet_host
                    .map(|host| format!("testnet with host '{host}'"))
                    .unwrap_or_else(|| "mainnet".to_string());
                debug!("Initialising Pubky singleton in {mode} mode");

                let client = match testnet_host {
                    Some(host) => PubkyHttpClient::builder().testnet_with_host(host).build(),
                    None => PubkyHttpClient::new(),
                }
                .map_err(|e| PubkyClientError::ClientError(e.to_string()))?;
                Ok(Arc::new(Pubky::with_client(client)))
            })
            .await
            .map(|_| ())
    }

    pub fn get() -> Result<Arc<Pubky>, PubkyClientError> {
        PUBKY_SINGLETON
            .get()
            .cloned()
            .ok_or(PubkyClientError::NotInitialized)
    }
}
