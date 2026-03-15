use std::collections::HashMap;

use mapky_app_specs::MapkyAppObject;
use mapky_common::db::{get_pg_pool, pg_queries, PubkyConnector};
use mapky_common::models::homeserver::HomeserverDetails;
use mapky_common::types::DynError;
use mapky_common::WatcherConfig;
use pubky::Method;
use tokio::sync::watch::Receiver;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use crate::events::{
    handle_del_event, handle_put_event, parse_event_line, parse_sse_events, EventLine, EventType,
};

/// Mapky.app path prefix for events-stream filtering.
const MAPKY_PATH_FILTER: &str = "/pub/mapky.app/";

/// Maximum users per events-stream request (Pubky limit is 50).
const MAX_USERS_PER_REQUEST: usize = 50;

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
            if let Err(e) = Self::poll_homeserver(config, hs_id).await {
                error!("Error polling homeserver {hs_id}: {e}");
            }
        }

        Ok(())
    }

    /// Poll a single homeserver — uses events-stream if we know users on it,
    /// falls back to legacy /events/ for discovery otherwise.
    async fn poll_homeserver(config: &WatcherConfig, homeserver: &str) -> Result<(), DynError> {
        let users = HomeserverDetails::get_user_ids(homeserver).await?;

        if users.is_empty() {
            // No known users on this homeserver — use legacy endpoint to discover them.
            debug!("No known users on {homeserver}, using legacy /events/ for discovery");
            Self::poll_events_legacy(config, homeserver).await
        } else {
            // Use events-stream with path filter for efficient polling.
            Self::poll_events_stream(config, homeserver, &users).await
        }
    }

    /// Poll via /events-stream with per-user cursors and mapky.app path filter.
    async fn poll_events_stream(
        config: &WatcherConfig,
        homeserver: &str,
        users: &[String],
    ) -> Result<(), DynError> {
        let pool = get_pg_pool()?;
        let pubky = PubkyConnector::get()?;

        // Build per-user cursor map
        let cursor_pairs = pg_queries::cursor::get_cursors_for_users(pool, users).await?;
        let cursor_map: HashMap<&str, &str> = cursor_pairs
            .iter()
            .map(|(id, cursor)| (id.as_str(), cursor.as_str()))
            .collect();

        // Process users in batches of MAX_USERS_PER_REQUEST
        for chunk in users.chunks(MAX_USERS_PER_REQUEST) {
            // Build URL: /events-stream?user=pk1:cursor1&user=pk2&path=/pub/mapky.app/&limit=N
            let mut url = format!("https://{homeserver}/events-stream?");
            for (i, user_id) in chunk.iter().enumerate() {
                if i > 0 {
                    url.push('&');
                }
                url.push_str("user=");
                url.push_str(user_id);
                if let Some(cursor) = cursor_map.get(user_id.as_str()) {
                    url.push(':');
                    url.push_str(cursor);
                }
            }
            url.push_str(&format!("&path={MAPKY_PATH_FILTER}&limit={}", config.events_limit));

            let response = pubky
                .client()
                .request(Method::GET, &url)
                .send()
                .await
                .map_err(|e| format!("Failed to poll events-stream on {homeserver}: {e}"))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!(
                    "events-stream error from {homeserver}: HTTP {status} — {body}"
                )
                .into());
            }

            let body = response.text().await?;
            if body.trim().is_empty() {
                debug!("No new events from {homeserver} via events-stream");
                continue;
            }

            let events = parse_sse_events(&body);
            if events.is_empty() {
                debug!("No mapky.app events from {homeserver} via events-stream");
                continue;
            }

            info!(
                "Processing {} events from {homeserver} via events-stream",
                events.len()
            );

            for result in events {
                match result {
                    Ok(event_line) => {
                        Self::process_event(&event_line, homeserver).await;

                        // Update per-user cursor
                        if let Some(ref cursor) = event_line.cursor {
                            if let Err(e) = pg_queries::cursor::upsert_cursor(
                                pool,
                                &event_line.user_id,
                                cursor,
                            )
                            .await
                            {
                                error!(
                                    "Failed to update cursor for user {}: {e}",
                                    event_line.user_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse SSE event: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Legacy polling via /events/ — used for homeservers where we have no known users.
    /// Discovers users and creates REGISTERED_ON links so future polls use events-stream.
    async fn poll_events_legacy(
        config: &WatcherConfig,
        homeserver: &str,
    ) -> Result<(), DynError> {
        let pool = get_pg_pool()?;
        let pubky = PubkyConnector::get()?;

        // Read homeserver-level cursor (keyed by homeserver ID)
        let cursor = pg_queries::cursor::get_cursor(pool, homeserver)
            .await?
            .unwrap_or_else(|| "0".to_string());

        let url = format!(
            "https://{homeserver}/events/?cursor={cursor}&limit={}",
            config.events_limit
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
            debug!("No new events from {homeserver} (legacy)");
            return Ok(());
        }

        info!(
            "Processing {} event lines from {homeserver} (legacy discovery)",
            lines.len()
        );

        for line in &lines {
            // Handle cursor update lines
            if let Some(new_cursor) = line.strip_prefix("cursor: ") {
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

            // Link user to homeserver so future polls use events-stream
            if let Err(e) =
                HomeserverDetails::link_user(&event_line.user_id, homeserver).await
            {
                error!(
                    "Failed to link user {} to homeserver {homeserver}: {e}",
                    event_line.user_id
                );
            }

            Self::process_event(&event_line, homeserver).await;
        }

        Ok(())
    }

    /// Process a single event (PUT or DEL) — shared between events-stream and legacy modes.
    async fn process_event(event_line: &EventLine, homeserver: &str) {
        match event_line.event_type {
            EventType::Put => {
                let pubky = match PubkyConnector::get() {
                    Ok(p) => p,
                    Err(e) => {
                        error!("PubkyConnector not available: {e}");
                        return;
                    }
                };

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
                        return;
                    }
                };

                if !response.status().is_success() {
                    error!(
                        "Fetch blob failed for {}: HTTP {}",
                        event_line.uri,
                        response.status()
                    );
                    return;
                }

                let blob = match response.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Failed to read blob for {}: {e}", event_line.uri);
                        return;
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
                        return;
                    }
                };

                if let Err(e) =
                    handle_put_event(object, &event_line.user_id, &event_line.resource_id).await
                {
                    error!("PUT handler error for {}: {e}", event_line.uri);
                }

                // Ensure user→homeserver link exists (idempotent)
                if let Err(e) =
                    HomeserverDetails::link_user(&event_line.user_id, homeserver).await
                {
                    error!(
                        "Failed to link user {} to homeserver: {e}",
                        event_line.user_id
                    );
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
}
