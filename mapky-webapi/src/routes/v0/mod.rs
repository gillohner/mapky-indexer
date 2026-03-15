mod health;
mod ingest;
mod viewport;

use axum::Router;
use utoipa::OpenApi;

use mapky_common::models::place::PlaceDetails;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        ingest::ingest,
        viewport::viewport,
    ),
    components(schemas(
        PlaceDetails,
        health::HealthResponse,
    ))
)]
pub struct ApiDoc;

pub fn routes() -> Router {
    Router::new()
        .merge(health::routes())
        .merge(ingest::routes())
        .merge(viewport::routes())
}
