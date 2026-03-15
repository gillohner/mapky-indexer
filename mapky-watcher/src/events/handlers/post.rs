use mapky_app_specs::MapkyAppPost;
use mapky_common::db::{execute_graph_operation, OperationOutcome};
use mapky_common::db::queries;
use mapky_common::models::place::PlaceDetails;
use mapky_common::models::post::PostDetails;
use mapky_common::types::DynError;
use tracing::{debug, warn};

/// Handle a PUT event for a post.
/// Flow: ensure Place exists → create PostDetails → put to graph → put to pg → update aggregates.
pub async fn sync_put(
    post: &MapkyAppPost,
    user_id: &str,
    post_id: &str,
) -> Result<(), DynError> {
    debug!("Indexing post: {}/{}", user_id, post_id);

    // Step 1: Ensure the place exists in the graph
    let place = PlaceDetails::from_osm_ref(&post.place);
    let place_query = queries::put::create_place(&place);
    execute_graph_operation(place_query).await?;

    // Step 2: Create PostDetails from the homeserver data
    let post_details = PostDetails::from_homeserver(post, user_id, post_id);

    // Step 3: Put the post into the graph
    let post_query = queries::put::create_post(&post_details);
    match execute_graph_operation(post_query).await? {
        OperationOutcome::CreatedOrDeleted => {
            debug!("Created new post {}/{}", user_id, post_id);
        }
        OperationOutcome::Updated => {
            debug!("Updated existing post {}/{}", user_id, post_id);
        }
        OperationOutcome::MissingDependency => {
            warn!(
                "Missing dependency for post {}/{} — user or place not indexed yet",
                user_id, post_id
            );
            // TODO: Queue for retry
            return Ok(());
        }
    }

    // Step 4: Put to PostgreSQL
    // TODO: Insert into posts table via sqlx

    // Step 5: Update place aggregates if this is a review (has rating)
    // TODO: UPDATE places SET review_count = review_count + 1, avg_rating = ... WHERE osm_canonical = ...

    Ok(())
}

/// Handle a DEL event for a post.
pub async fn del(user_id: &str, post_id: &str) -> Result<(), DynError> {
    debug!("Deleting post: {}/{}", user_id, post_id);

    let query = queries::del::delete_post(user_id, post_id);
    execute_graph_operation(query).await?;

    // TODO: Delete from PostgreSQL posts table
    // TODO: Update place aggregates

    Ok(())
}
