use crate::db::{exec_single_row, fetch_key_from_graph, queries, PubkyConnector};
use crate::types::DynError;
use pubky::PublicKey;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use utoipa::ToSchema;

/// Indexed representation of a Pubky homeserver that the watcher should poll.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HomeserverDetails {
    /// Homeserver public key.
    pub id: String,
    /// Unix timestamp (millis) when this homeserver was first indexed.
    pub indexed_at: i64,
}

impl HomeserverDetails {
    pub fn new(id: String) -> Self {
        let indexed_at = chrono::Utc::now().timestamp_millis();
        Self { id, indexed_at }
    }

    /// MERGE the homeserver into Neo4j if it doesn't already exist.
    pub async fn persist_if_unknown(id: &str) -> Result<(), DynError> {
        let existing: Option<String> =
            fetch_key_from_graph(queries::get::get_homeserver_by_id(id), "id").await?;

        if existing.is_some() {
            debug!("Homeserver {id} already known, skipping");
            return Ok(());
        }

        let hs = Self::new(id.to_string());
        exec_single_row(queries::put::create_homeserver(&hs)).await?;
        info!("Persisted new homeserver: {id}");
        Ok(())
    }

    /// Resolve a user's homeserver via DHT and persist it if unknown.
    /// Skips if the user's homeserver is already tracked.
    pub async fn maybe_ingest_for_user(user_id: &str) -> Result<(), DynError> {
        let user_pk = PublicKey::try_from(user_id)
            .map_err(|e| format!("Invalid public key '{user_id}': {e}"))?;

        let pubky = PubkyConnector::get()?;

        let homeserver = pubky
            .get_homeserver_of(&user_pk)
            .await
            .ok_or_else(|| format!("Could not resolve homeserver for {user_id}"))?;

        let hs_id = homeserver.to_string();
        info!("Resolved homeserver for {user_id}: {hs_id}");

        Self::persist_if_unknown(&hs_id).await
    }

    /// Return all known homeserver IDs from the graph.
    pub async fn get_all_ids() -> Result<Vec<String>, DynError> {
        let ids: Option<Vec<String>> =
            fetch_key_from_graph(queries::get::get_all_homeservers(), "ids").await?;

        Ok(ids.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homeserver_details_new() {
        let hs = HomeserverDetails::new("test_hs_pk".to_string());
        assert_eq!(hs.id, "test_hs_pk");
        assert!(hs.indexed_at > 0);
    }

    #[test]
    fn test_homeserver_details_serde_roundtrip() {
        let hs = HomeserverDetails::new("test_hs_pk".to_string());
        let json = serde_json::to_string(&hs).unwrap();
        let parsed: HomeserverDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, hs.id);
        assert_eq!(parsed.indexed_at, hs.indexed_at);
    }
}
