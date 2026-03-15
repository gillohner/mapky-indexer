use once_cell::sync::OnceCell;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{debug, info};

use crate::config::PostgresConfig;
use crate::types::DynError;

pub struct PgConnector;

impl PgConnector {
    pub async fn init(pg_config: &PostgresConfig) -> Result<(), DynError> {
        let pool = PgPoolOptions::new()
            .max_connections(pg_config.max_connections)
            .connect(&pg_config.url)
            .await
            .map_err(|e| format!("Could not connect to PostgreSQL: {e}"))?;

        info!("Connected to PostgreSQL at {}", pg_config.url);

        // Run migrations
        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Failed to run PostgreSQL migrations: {e}"))?;

        info!("PostgreSQL migrations applied successfully");

        match PG_CONNECTOR.set(pool) {
            Err(_) => debug!("PgConnector was already set"),
            Ok(()) => info!("PgConnector successfully set up"),
        }

        Ok(())
    }
}

pub fn get_pg_pool() -> Result<&'static PgPool, &'static str> {
    PG_CONNECTOR.get().ok_or("PgConnector not initialized")
}

pub static PG_CONNECTOR: OnceCell<PgPool> = OnceCell::new();
