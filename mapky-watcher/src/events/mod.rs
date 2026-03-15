pub mod handlers;

use mapky_app_specs::MapkyAppObject;
use mapky_common::types::DynError;
use tracing::debug;

/// A parsed event line from the homeserver /events/ endpoint.
#[derive(Debug)]
pub struct EventLine {
    pub event_type: EventType,
    pub uri: String,
    pub user_id: String,
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Debug, PartialEq)]
pub enum EventType {
    Put,
    Del,
}

const MAPKY_APP_PREFIX: &str = "pub/mapky.app/";

/// Parse a single event line from the homeserver feed.
/// Returns Ok(None) for non-mapky.app events (skipped).
/// Returns Err for malformed lines.
pub fn parse_event_line(line: &str) -> Result<Option<EventLine>, DynError> {
    let (type_str, uri) = line
        .split_once(' ')
        .ok_or_else(|| format!("Malformed event line: {line}"))?;

    let event_type = match type_str {
        "PUT" => EventType::Put,
        "DEL" => EventType::Del,
        other => return Err(format!("Unknown event type: {other}").into()),
    };

    // Parse pubky://user_pk/pub/mapky.app/resource_type/id
    let stripped = uri
        .strip_prefix("pubky://")
        .ok_or_else(|| format!("URI missing pubky:// prefix: {uri}"))?;

    // Split into user_id and the rest of the path
    let (user_id, path) = stripped
        .split_once('/')
        .ok_or_else(|| format!("URI missing path: {uri}"))?;

    // Check if this is a mapky.app path
    let mapky_path = match path.strip_prefix(MAPKY_APP_PREFIX) {
        Some(rest) => rest,
        None => return Ok(None), // Not a mapky.app event, skip
    };

    let (resource_type, resource_id) = mapky_path
        .split_once('/')
        .ok_or_else(|| format!("URI missing resource id: {uri}"))?;

    Ok(Some(EventLine {
        event_type,
        uri: uri.to_string(),
        user_id: user_id.to_string(),
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
    }))
}

/// Dispatch a PUT event to the appropriate handler based on the parsed object.
pub async fn handle_put_event(
    object: MapkyAppObject,
    user_id: &str,
    id: &str,
) -> Result<(), DynError> {
    match object {
        MapkyAppObject::Post(post) => {
            handlers::post::sync_put(&post, user_id, id).await?;
        }
        other => {
            debug!("PUT handler not yet implemented for: {:?}", other);
        }
    }
    Ok(())
}

/// Dispatch a DEL event based on the resource path segment.
pub async fn handle_del_event(
    path_segment: &str,
    user_id: &str,
    id: &str,
) -> Result<(), DynError> {
    match path_segment {
        "posts" => {
            handlers::post::del(user_id, id).await?;
        }
        other => {
            debug!("DEL handler not yet implemented for: {other}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_put_event() {
        let line = "PUT pubky://abc123/pub/mapky.app/posts/0034A0X7NJ52G";
        let event = parse_event_line(line).unwrap().unwrap();
        assert_eq!(event.event_type, EventType::Put);
        assert_eq!(event.user_id, "abc123");
        assert_eq!(event.resource_type, "posts");
        assert_eq!(event.resource_id, "0034A0X7NJ52G");
        assert_eq!(event.uri, "pubky://abc123/pub/mapky.app/posts/0034A0X7NJ52G");
    }

    #[test]
    fn test_parse_del_event() {
        let line = "DEL pubky://user1/pub/mapky.app/location_tags/ABCDEF";
        let event = parse_event_line(line).unwrap().unwrap();
        assert_eq!(event.event_type, EventType::Del);
        assert_eq!(event.user_id, "user1");
        assert_eq!(event.resource_type, "location_tags");
        assert_eq!(event.resource_id, "ABCDEF");
    }

    #[test]
    fn test_parse_non_mapky_event_returns_none() {
        let line = "PUT pubky://abc123/pub/pubky.app/posts/001";
        let result = parse_event_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_malformed_line() {
        let result = parse_event_line("GARBAGE");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_event_type() {
        let result = parse_event_line("PATCH pubky://abc/pub/mapky.app/posts/001");
        assert!(result.is_err());
    }
}
