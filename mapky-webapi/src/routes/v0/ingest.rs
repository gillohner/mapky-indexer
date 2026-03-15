use axum::extract::Path;
use axum::routing::put;
use axum::{Json, Router};

use mapky_common::models::homeserver::HomeserverDetails;

use crate::error::Error;

#[utoipa::path(
    put,
    path = "/v0/ingest/{user_id}",
    params(
        ("user_id" = String, Path, description = "Public key of the user whose homeserver should be ingested")
    ),
    responses(
        (status = 200, description = "Homeserver ingested successfully"),
        (status = 400, description = "Invalid user ID"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn ingest(Path(user_id): Path<String>) -> Result<Json<serde_json::Value>, Error> {
    if user_id.is_empty() {
        return Err(Error::invalid_input("user_id must not be empty"));
    }

    HomeserverDetails::maybe_ingest_for_user(&user_id)
        .await
        .map_err(Error::internal)?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub fn routes() -> Router {
    Router::new().route("/v0/ingest/{user_id}", put(ingest))
}
