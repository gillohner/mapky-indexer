use axum::extract::Path;
use axum::routing::put;
use axum::{Json, Router};

use mapky_common::db::{exec_single_row, queries, PubkyConnector};
use mapky_common::models::homeserver::HomeserverDetails;
use mapky_common::models::user::UserDetails;

use crate::error::Error;

#[utoipa::path(
    put,
    path = "/v0/ingest/{user_id}",
    params(
        ("user_id" = String, Path, description = "Public key of the user whose homeserver should be ingested")
    ),
    responses(
        (status = 200, description = "User and homeserver ingested successfully"),
        (status = 400, description = "Invalid user ID"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn ingest(Path(user_id): Path<String>) -> Result<Json<serde_json::Value>, Error> {
    if user_id.is_empty() {
        return Err(Error::invalid_input("user_id must not be empty"));
    }

    // Resolve the user's homeserver via DHT
    let user_pk = pubky::PublicKey::try_from(user_id.as_str())
        .map_err(|e| Error::invalid_input(&format!("Invalid public key: {e}")))?;

    let pubky = PubkyConnector::get()
        .map_err(|e| Error::internal(e.to_string().into()))?;

    let homeserver = pubky
        .get_homeserver_of(&user_pk)
        .await
        .ok_or_else(|| {
            Error::internal(format!("Could not resolve homeserver for {user_id}").into())
        })?;

    let hs_id = homeserver.to_string();

    // Persist homeserver if new
    HomeserverDetails::persist_if_unknown(&hs_id)
        .await
        .map_err(Error::internal)?;

    // Create user node and link to homeserver
    let user = UserDetails {
        id: user_id.clone(),
        name: user_id.clone(), // placeholder until profile data available
        indexed_at: chrono::Utc::now().timestamp_millis(),
    };
    exec_single_row(queries::put::create_user(&user))
        .await
        .map_err(Error::internal)?;

    HomeserverDetails::link_user(&user_id, &hs_id)
        .await
        .map_err(Error::internal)?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "homeserver": hs_id
    })))
}

pub fn routes() -> Router {
    Router::new().route("/v0/ingest/{user_id}", put(ingest))
}
