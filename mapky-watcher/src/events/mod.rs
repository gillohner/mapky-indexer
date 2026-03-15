pub mod handlers;

use mapky_app_specs::MapkyAppObject;
use mapky_common::types::DynError;
use tracing::debug;

/// A parsed event line from the homeserver /events/ endpoint (legacy plain text)
/// or /events-stream endpoint (SSE).
#[derive(Debug)]
pub struct EventLine {
    pub event_type: EventType,
    pub uri: String,
    pub user_id: String,
    pub resource_type: String,
    pub resource_id: String,
    /// Per-event cursor from events-stream (None for legacy endpoint).
    pub cursor: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum EventType {
    Put,
    Del,
}

const MAPKY_APP_PREFIX: &str = "pub/mapky.app/";

/// Parse a single event line from the legacy homeserver /events/ feed.
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

    parse_uri(uri, event_type, None)
}

/// Parse a pubky:// URI into an EventLine.
fn parse_uri(
    uri: &str,
    event_type: EventType,
    cursor: Option<String>,
) -> Result<Option<EventLine>, DynError> {
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
        cursor,
    }))
}

/// Parse a Server-Sent Events (SSE) response body into EventLines.
///
/// SSE format from /events-stream:
/// ```text
/// event: PUT
/// data: pubky://user_pk/pub/mapky.app/posts/ID
/// data: cursor: 42
/// data: content_hash: abc123...
///
/// event: DEL
/// data: pubky://user_pk/pub/mapky.app/posts/ID2
/// data: cursor: 43
/// ```
pub fn parse_sse_events(body: &str) -> Vec<Result<EventLine, DynError>> {
    let mut results = Vec::new();

    // SSE events are separated by blank lines
    for block in body.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut event_type: Option<EventType> = None;
        let mut uri: Option<String> = None;
        let mut cursor: Option<String> = None;

        for line in block.lines() {
            let line = line.trim();
            if let Some(type_str) = line.strip_prefix("event: ") {
                event_type = match type_str {
                    "PUT" => Some(EventType::Put),
                    "DEL" => Some(EventType::Del),
                    _ => None,
                };
            } else if let Some(data) = line.strip_prefix("data: ") {
                if data.starts_with("pubky://") {
                    uri = Some(data.to_string());
                } else if let Some(c) = data.strip_prefix("cursor: ") {
                    cursor = Some(c.to_string());
                }
                // content_hash lines are ignored (not needed for indexing)
            }
        }

        let Some(et) = event_type else {
            continue; // skip blocks without a recognized event type (e.g. keep-alive comments)
        };
        let Some(u) = uri else {
            results.push(Err("SSE event block missing URI data line".into()));
            continue;
        };

        match parse_uri(&u, et, cursor) {
            Ok(Some(event_line)) => results.push(Ok(event_line)),
            Ok(None) => {} // non-mapky.app event, skip (shouldn't happen with path filter)
            Err(e) => results.push(Err(e)),
        }
    }

    results
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
        assert!(event.cursor.is_none());
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

    #[test]
    fn test_parse_sse_put_event() {
        let body = "event: PUT\ndata: pubky://user1/pub/mapky.app/posts/001\ndata: cursor: 42\ndata: content_hash: abc123\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        assert_eq!(event.event_type, EventType::Put);
        assert_eq!(event.user_id, "user1");
        assert_eq!(event.resource_type, "posts");
        assert_eq!(event.resource_id, "001");
        assert_eq!(event.cursor.as_deref(), Some("42"));
    }

    #[test]
    fn test_parse_sse_del_event() {
        let body = "event: DEL\ndata: pubky://user1/pub/mapky.app/posts/002\ndata: cursor: 43\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        assert_eq!(event.event_type, EventType::Del);
        assert_eq!(event.cursor.as_deref(), Some("43"));
    }

    #[test]
    fn test_parse_sse_multiple_events() {
        let body = "\
event: PUT\n\
data: pubky://u1/pub/mapky.app/posts/001\n\
data: cursor: 10\n\
data: content_hash: hash1\n\
\n\
event: DEL\n\
data: pubky://u2/pub/mapky.app/posts/002\n\
data: cursor: 11\n\
\n\
event: PUT\n\
data: pubky://u1/pub/mapky.app/posts/003\n\
data: cursor: 12\n\
data: content_hash: hash3\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].as_ref().unwrap().event_type, EventType::Put);
        assert_eq!(events[1].as_ref().unwrap().event_type, EventType::Del);
        assert_eq!(events[2].as_ref().unwrap().event_type, EventType::Put);
    }

    #[test]
    fn test_parse_sse_skips_non_mapky_events() {
        let body = "event: PUT\ndata: pubky://u1/pub/pubky.app/posts/001\ndata: cursor: 10\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 0); // filtered out
    }

    #[test]
    fn test_parse_sse_empty_body() {
        let events = parse_sse_events("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_sse_keeps_alive_comments() {
        // SSE keep-alive comments start with ':' — should be ignored
        let body = ": keep-alive\n\nevent: PUT\ndata: pubky://u1/pub/mapky.app/posts/001\ndata: cursor: 1\n\n";
        let events = parse_sse_events(body);
        assert_eq!(events.len(), 1);
    }
}
