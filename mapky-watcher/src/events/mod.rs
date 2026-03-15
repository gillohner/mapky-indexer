pub mod handlers;

use mapky_app_specs::MapkyAppObject;
use mapky_common::types::DynError;
use tracing::debug;

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
