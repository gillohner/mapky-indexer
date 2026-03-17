use axum::extract::{Path, Query};
use axum::routing::get;
use axum::{Json, Router};

use mapky_common::db::{fetch_all_rows_from_graph, get_pg_pool, pg_queries, queries};
use mapky_common::models::place::PlaceDetails;
use mapky_common::models::post::PostDetails;
use mapky_common::types::PlacePostsQuery;

use crate::error::Error;

const VALID_OSM_TYPES: &[&str] = &["node", "way", "relation"];

fn validate_osm_type(osm_type: &str) -> Result<(), Error> {
    if !VALID_OSM_TYPES.contains(&osm_type) {
        return Err(Error::invalid_input(&format!(
            "osm_type must be one of: node, way, relation (got '{osm_type}')"
        )));
    }
    Ok(())
}

fn make_canonical(osm_type: &str, osm_id: i64) -> String {
    format!("{osm_type}/{osm_id}")
}

/// Get a place by OSM type and ID.
#[utoipa::path(
    get,
    path = "/v0/place/{osm_type}/{osm_id}",
    params(
        ("osm_type" = String, Path, description = "OSM element type: node, way, or relation"),
        ("osm_id" = i64, Path, description = "OSM element ID"),
    ),
    responses(
        (status = 200, description = "Place details", body = PlaceDetails),
        (status = 400, description = "Invalid OSM type"),
        (status = 404, description = "Place not found"),
    )
)]
pub async fn place_detail(
    Path((osm_type, osm_id)): Path<(String, i64)>,
) -> Result<Json<PlaceDetails>, Error> {
    validate_osm_type(&osm_type)?;
    let canonical = make_canonical(&osm_type, osm_id);

    // Try PostgreSQL first (has aggregation data)
    let pool = get_pg_pool().map_err(|e| Error::internal(e.to_string().into()))?;
    if let Some(place) = pg_queries::place::get_place_by_canonical(pool, &canonical)
        .await
        .map_err(Error::internal)?
    {
        return Ok(Json(place));
    }

    // Fallback: check Neo4j (place might exist in graph but not yet in PG)
    let query = queries::get::get_place_by_canonical(&canonical);
    let rows = fetch_all_rows_from_graph(query)
        .await
        .map_err(Error::internal)?;

    match rows.into_iter().next() {
        Some(row) => {
            let place: PlaceDetails = row.get("place").map_err(|e| {
                Error::internal(format!("Failed to deserialize place: {e}").into())
            })?;
            Ok(Json(place))
        }
        None => Err(Error::PlaceNotFound {
            osm_canonical: canonical,
        }),
    }
}

/// List posts for a place, newest first.
#[utoipa::path(
    get,
    path = "/v0/place/{osm_type}/{osm_id}/posts",
    params(
        ("osm_type" = String, Path, description = "OSM element type: node, way, or relation"),
        ("osm_id" = i64, Path, description = "OSM element ID"),
        PlacePostsQuery,
    ),
    responses(
        (status = 200, description = "Posts for this place", body = Vec<PostDetails>),
        (status = 400, description = "Invalid OSM type or parameters"),
    )
)]
pub async fn place_posts(
    Path((osm_type, osm_id)): Path<(String, i64)>,
    Query(params): Query<PlacePostsQuery>,
) -> Result<Json<Vec<PostDetails>>, Error> {
    validate_osm_type(&osm_type)?;
    let canonical = make_canonical(&osm_type, osm_id);

    let pool = get_pg_pool().map_err(|e| Error::internal(e.to_string().into()))?;
    let posts = pg_queries::post::get_posts_for_place(
        pool,
        &canonical,
        params.reviews_only(),
        params.skip(),
        params.limit(),
    )
    .await
    .map_err(Error::internal)?;

    Ok(Json(posts))
}

pub fn routes() -> Router {
    Router::new()
        .route("/v0/place/{osm_type}/{osm_id}", get(place_detail))
        .route("/v0/place/{osm_type}/{osm_id}/posts", get(place_posts))
}
