use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DbError {
    #[error("GraphQueryFailed: {message}")]
    GraphQueryFailed { message: String },
    #[error("PostgresOperationFailed: {message}")]
    PostgresOperationFailed { message: String },
}
