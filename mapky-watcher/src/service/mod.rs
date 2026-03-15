use mapky_app_specs::MapkyAppObject;
use mapky_common::db::{get_pg_pool, pg_queries, PubkyConnector};
use mapky_common::models::homeserver::HomeserverDetails;
use mapky_common::types::DynError;
use mapky_common::WatcherConfig;
use pubky::Method;
use tokio::sync::watch::Receiver;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use crate::events::{handle_del_event, handle_put_event, parse_event_line, EventType};

pub struct MapkyWatcher;

impl MapkyWatcher {
    pub async fn start(
        mut shutdown_rx: Receiver<bool>,
        config: WatcherConfig,
    ) -> Result<(), DynError> {
        debug!(?config, "Running MapkyWatcher");

        // Ensure the configured (seed) homeserver is persisted in Neo4j
        // so the watcher always has at least one homeserver to poll.
        HomeserverDetails::persist_if_unknown(&config.homeserver).await?;
        info!(
            "Seed homeserver {} registered in graph",
            config.homeserver
        );

        let mut interval = tokio::time::interval(Duration::from_millis(config.watcher_sleep));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("SIGINT received, exiting MapKy Watcher loop");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = Self::poll_all_homeservers(&config).await {
                        error!("Error polling homeservers: {e}");
                    }
                }
            }
        }
        info!("MapKy Watcher shut down gracefully");
        Ok(())
    }

    /// Poll events from all known homeservers.
    async fn poll_all_homeservers(config: &WatcherConfig) -> Result<(), DynError> {
        let mut homeservers = HomeserverDetails::get_all_ids().await?;

        if homeservers.is_empty() {
            warn!("No homeservers in graph, falling back to seed homeserver");
            homeservers.push(config.homeserver.clone());
        }

        // Prioritise the seed/default homeserver by moving it to the front.
        if let Some(pos) = homeservers.iter().position(|id| id == &config.homeserver) {
            if pos != 0 {
                homeservers.swap(0, pos);
            }
        }

        for hs_id in &homeservers {
            if let Err(e) = Self::poll_events(config, hs_id).await {
                error!("Error polling homeserver {hs_id}: {e}");
            }
        }

        Ok(())
    }

    /// Poll events from a single homeserver.
    async fn poll_events(config: &WatcherConfig, homeserver: &str) -> Result<(), DynError> {
        let pool = get_pg_pool()?;

        // 1. Read cursor from PostgreSQL (default "0" on first run)
        let cursor = pg_queries::cursor::get_cursor(pool, homeserver)
            .await?
            .unwrap_or_else(|| "0".to_string());

        // 2. Fetch events from homeserver
        let pubky = PubkyConnector::get()?;
        let url = format!(
            "https://{}/events/?cursor={}&limit={}",
            homeserver, cursor, config.events_limit
        );

        let response = pubky
            .client()
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| format!("Failed to poll homeserver {homeserver}: {e}"))?;

        let response_text = response.text().await?;
        let lines: Vec<&str> = response_text.trim().lines().collect();

        if lines.is_empty() || (lines.len() == 1 && lines[0].is_empty()) {
            debug!("No new events from {homeserver}");
            return Ok(());
        }

        info!("Processing {} event lines from {homeserver}", lines.len());

        // 3. Process each line
        for line in &lines {
            // Handle cursor update lines
            if let Some(new_cursor) = line.strip_prefix("cursor: ") {
                info!("Updating cursor for {homeserver} to: {new_cursor}");
                pg_queries::cursor::upsert_cursor(pool, homeserver, new_cursor).await?;
                continue;
            }

            // Parse the event line
            let event_line = match parse_event_line(line) {
                Ok(Some(el)) => el,
                Ok(None) => {
                    debug!("Skipping non-mapky.app event: {line}");
                    continue;
                }
                Err(e) => {
                    error!("Failed to parse event line '{line}': {e}");
                    continue;
                }
            };

            // Dispatch based on event type
            match event_line.event_type {
                EventType::Put => {
                    // Fetch the resource blob from the homeserver
                    let response = match pubky
                        .client()
                        .request(Method::GET, &event_line.uri)
                        .send()
                        .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Failed to fetch blob for {}: {e}", event_line.uri);
                            continue;
                        }
                    };

                    if !response.status().is_success() {
                        error!(
                            "Fetch blob failed for {}: HTTP {}",
                            event_line.uri,
                            response.status()
                        );
                        continue;
                    }

                    let blob = match response.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            error!("Failed to read blob for {}: {e}", event_line.uri);
                            continue;
                        }
                    };

                    // Parse into typed object
                    let object = match MapkyAppObject::from_path(
                        &event_line.resource_type,
                        &blob,
                        &event_line.resource_id,
                    ) {
                        Ok(obj) => obj,
                        Err(e) => {
                            error!("Failed to parse object for {}: {e}", event_line.uri);
                            continue;
                        }
                    };

                    if let Err(e) =
                        handle_put_event(object, &event_line.user_id, &event_line.resource_id)
                            .await
                    {
                        error!("PUT handler error for {}: {e}", event_line.uri);
                    }
                }
                EventType::Del => {
                    if let Err(e) = handle_del_event(
                        &event_line.resource_type,
                        &event_line.user_id,
                        &event_line.resource_id,
                    )
                    .await
                    {
                        error!("DEL handler error for {}: {e}", event_line.uri);
                    }
                }
            }
        }

        Ok(())
    }
}
