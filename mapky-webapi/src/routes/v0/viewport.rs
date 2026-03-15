use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};

use mapky_common::db::{fetch_all_rows_from_graph, queries};
use mapky_common::models::place::PlaceDetails;
use mapky_common::types::ViewportQuery;

use crate::error::Error;

#[utoipa::path(
    get,
    path = "/v0/viewport",
    params(ViewportQuery),
    responses(
        (status = 200, description = "Places within viewport", body = Vec<PlaceDetails>),
        (status = 400, description = "Invalid viewport parameters"),
    )
)]
pub async fn viewport(
    Query(params): Query<ViewportQuery>,
) -> Result<Json<Vec<PlaceDetails>>, Error> {
    params
        .validate()
        .map_err(|e| Error::invalid_input(&e))?;

    let query = queries::get::get_places_in_viewport(
        params.min_lat,
        params.min_lon,
        params.max_lat,
        params.max_lon,
        params.limit,
    );

    let rows = fetch_all_rows_from_graph(query)
        .await
        .map_err(Error::internal)?;

    let places: Vec<PlaceDetails> = rows
        .into_iter()
        .filter_map(|row| row.get("place").ok())
        .collect();

    Ok(Json(places))
}

pub fn routes() -> Router {
    Router::new().route("/v0/viewport", get(viewport))
}
