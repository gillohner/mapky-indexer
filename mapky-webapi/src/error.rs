use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use mapky_common::types::DynError;
use thiserror::Error;
use tracing::error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Place not found: {osm_canonical}")]
    PlaceNotFound { osm_canonical: String },
    #[error("Post not found: {author_id}/{post_id}")]
    PostNotFound { author_id: String, post_id: String },
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },
    #[error("Internal server error: {source}")]
    InternalServerError { source: Box<dyn std::error::Error> },
}

impl Error {
    pub fn internal(source: DynError) -> Self {
        Error::InternalServerError { source }
    }

    pub fn invalid_input(message: &str) -> Self {
        Error::InvalidInput {
            message: message.to_string(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status_code = match &self {
            Error::PlaceNotFound { .. } => StatusCode::NOT_FOUND,
            Error::PostNotFound { .. } => StatusCode::NOT_FOUND,
            Error::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            Error::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };

        match &self {
            Error::PlaceNotFound { osm_canonical } => {
                error!("Place not found: {osm_canonical}")
            }
            Error::PostNotFound {
                author_id,
                post_id,
            } => error!("Post not found: {author_id}/{post_id}"),
            Error::InvalidInput { message } => error!("Invalid input: {message}"),
            Error::InternalServerError { source } => error!("Internal error: {source:?}"),
        }

        let body = serde_json::json!({
            "error": self.to_string()
        });

        (status_code, axum::Json(body)).into_response()
    }
}
