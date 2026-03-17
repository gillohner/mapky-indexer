use mapky_app_specs::MapkyAppPost;
use mapky_common::db::{execute_graph_operation, get_pg_pool, pg_queries, OperationOutcome};
use mapky_common::db::queries;
use mapky_common::models::place::PlaceDetails;
use mapky_common::models::post::PostDetails;
use mapky_common::models::user::UserDetails;
use mapky_common::types::DynError;
use tracing::{debug, warn};

/// Handle a PUT event for a post.
/// Flow: ensure User exists → ensure Place exists → create PostDetails → put to graph → put to pg → update aggregates.
pub async fn sync_put(
    post: &MapkyAppPost,
    user_id: &str,
    post_id: &str,
) -> Result<(), DynError> {
    debug!("Indexing post: {}/{}", user_id, post_id);

    // Step 1: Ensure the user exists in the graph
    let user = UserDetails {
        id: user_id.to_string(),
        name: user_id.to_string(), // placeholder — no profile data available from post events
        indexed_at: chrono::Utc::now().timestamp_millis(),
    };
    let user_query = queries::put::create_user(&user);
    execute_graph_operation(user_query).await?;

    // Step 2: Ensure the place exists in the graph (resolves coordinates via Nominatim)
    let place = PlaceDetails::from_osm_ref(&post.place).await;
    let place_query = queries::put::create_place(&place);
    execute_graph_operation(place_query).await?;

    // Step 3: Create PostDetails from the homeserver data
    let post_details = PostDetails::from_homeserver(post, user_id, post_id);

    // Step 4: Put the post into the graph
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
                "Unexpected missing dependency for post {}/{} — user and place should have been auto-created",
                user_id, post_id
            );
            return Ok(());
        }
    }

    // Step 5: Put to PostgreSQL (place + post)
    let pool = get_pg_pool()?;
    pg_queries::place::upsert_place(pool, &place).await?;
    pg_queries::post::upsert_post(pool, &post_details).await?;

    // Step 6: Update place aggregates if this is a review (has rating)
    if let Some(rating) = post_details.rating {
        pg_queries::place::increment_review(pool, &post_details.osm_canonical, rating).await?;
    }

    Ok(())
}

/// Handle a DEL event for a post.
pub async fn del(user_id: &str, post_id: &str) -> Result<(), DynError> {
    debug!("Deleting post: {}/{}", user_id, post_id);

    let query = queries::del::delete_post(user_id, post_id);
    execute_graph_operation(query).await?;

    // Delete from PostgreSQL and get old data for aggregate rollback
    let pool = get_pg_pool()?;
    let deleted = pg_queries::post::delete_post(pool, user_id, post_id).await?;

    // Update place aggregates if the deleted post had a rating
    if let Some((osm_canonical, Some(rating))) = deleted {
        pg_queries::place::decrement_review(pool, &osm_canonical, rating as u8).await?;
    }

    Ok(())
}
