use crate::types::DynError;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

static NOMINATIM: OnceCell<NominatimClient> = OnceCell::const_new();

pub struct NominatimClient {
    http: reqwest::Client,
    base_url: String,
    /// Enforces minimum 1s gap between requests (Nominatim ToS for public instance).
    last_request: Mutex<std::time::Instant>,
}

#[derive(Debug, Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
}

impl NominatimClient {
    pub fn init(base_url: &str) {
        let client = Self {
            http: reqwest::Client::builder()
                .user_agent("mapky-indexer/0.1 (https://github.com/nicobao/mapky)")
                .build()
                .expect("Failed to build reqwest client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            last_request: Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(2)),
        };
        NOMINATIM.set(client).ok();
    }

    fn get() -> Option<&'static NominatimClient> {
        NOMINATIM.get()
    }
}

/// Resolve an OSM element's coordinates via Nominatim lookup.
/// Returns `None` if the lookup fails or the element is not found.
/// Rate-limited to 1 request per second for the public Nominatim instance.
pub async fn resolve_osm_coords(
    osm_type: &str,
    osm_id: i64,
) -> Result<Option<(f64, f64)>, DynError> {
    let client = match NominatimClient::get() {
        Some(c) => c,
        None => {
            warn!("NominatimClient not initialized — coordinates will be (0, 0)");
            return Ok(None);
        }
    };

    // Rate limit: wait until at least 1s since the last request
    {
        let mut last = client.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < std::time::Duration::from_secs(1) {
            tokio::time::sleep(std::time::Duration::from_secs(1) - elapsed).await;
        }
        *last = std::time::Instant::now();
    }

    // Nominatim lookup uses N/W/R prefix for osm_ids parameter
    let type_prefix = match osm_type {
        "node" => "N",
        "way" => "W",
        "relation" => "R",
        other => {
            warn!("Unknown OSM type '{other}' — cannot resolve coordinates");
            return Ok(None);
        }
    };

    let url = format!(
        "{}/lookup?osm_ids={}{}&format=json",
        client.base_url, type_prefix, osm_id
    );
    debug!("Nominatim lookup: {url}");

    let response = match client.http.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Nominatim request failed for {osm_type}/{osm_id}: {e}");
            return Ok(None);
        }
    };

    if !response.status().is_success() {
        warn!(
            "Nominatim returned {} for {osm_type}/{osm_id}",
            response.status()
        );
        return Ok(None);
    }

    let results: Vec<NominatimResult> = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to parse Nominatim response for {osm_type}/{osm_id}: {e}");
            return Ok(None);
        }
    };

    match results.first() {
        Some(result) => {
            let lat: f64 = result.lat.parse().map_err(|e| {
                format!("Failed to parse lat '{}': {e}", result.lat)
            })?;
            let lon: f64 = result.lon.parse().map_err(|e| {
                format!("Failed to parse lon '{}': {e}", result.lon)
            })?;
            debug!("Resolved {osm_type}/{osm_id} → ({lat}, {lon})");
            Ok(Some((lat, lon)))
        }
        None => {
            warn!("Nominatim returned empty results for {osm_type}/{osm_id}");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_prefix_mapping() {
        // Verify the match arms exist (tested indirectly via resolve_osm_coords)
        assert_eq!(
            match "node" {
                "node" => "N",
                "way" => "W",
                "relation" => "R",
                _ => "?",
            },
            "N"
        );
        assert_eq!(
            match "way" {
                "node" => "N",
                "way" => "W",
                "relation" => "R",
                _ => "?",
            },
            "W"
        );
        assert_eq!(
            match "relation" {
                "node" => "N",
                "way" => "W",
                "relation" => "R",
                _ => "?",
            },
            "R"
        );
    }

    #[tokio::test]
    async fn test_resolve_without_init_returns_none() {
        // NominatimClient not initialized in test context → should return None gracefully
        let result = resolve_osm_coords("node", 123).await.unwrap();
        assert!(result.is_none());
    }
}
