mod health;
mod ingest;
mod place;
mod viewport;

use axum::Router;
use utoipa::OpenApi;

use mapky_common::models::place::PlaceDetails;
use mapky_common::models::post::PostDetails;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        ingest::ingest,
        viewport::viewport,
        place::place_detail,
        place::place_posts,
    ),
    components(schemas(
        PlaceDetails,
        PostDetails,
        health::HealthResponse,
    ))
)]
pub struct ApiDoc;

pub fn routes() -> Router {
    Router::new()
        .merge(health::routes())
        .merge(ingest::routes())
        .merge(viewport::routes())
        .merge(place::routes())
}
