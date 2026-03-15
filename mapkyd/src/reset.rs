use std::path::PathBuf;

use mapky_common::db::graph::setup::setup_graph;
use mapky_common::db::{get_neo4j_graph, get_pg_pool};
use mapky_common::types::DynError;
use mapky_common::{DaemonConfig, StackManager};
use neo4rs::query;
use tracing::info;

/// Wipe database contents and recreate schema.
///
/// Loads config from `config_dir`, initializes DB connectors, then:
/// - Neo4j: `MATCH (n) DETACH DELETE n` + re-run `setup_graph()`
/// - PostgreSQL: `TRUNCATE` all tables + re-run migrations
pub async fn reset_databases(
    config_dir: PathBuf,
    neo4j_only: bool,
    pg_only: bool,
) -> Result<(), DynError> {
    let config = DaemonConfig::read_or_create_config_file(config_dir).await?;
    StackManager::setup("mapkyd.reset", &config.stack).await?;

    let reset_neo4j = !pg_only;
    let reset_pg = !neo4j_only;

    if reset_neo4j {
        do_reset_neo4j().await?;
    }

    if reset_pg {
        do_reset_postgres().await?;
    }

    info!("Database reset complete");
    Ok(())
}

/// Delete all nodes and relationships, then recreate constraints and indexes.
async fn do_reset_neo4j() -> Result<(), DynError> {
    let graph = get_neo4j_graph()?;
    let graph = graph.lock().await;

    graph
        .run(query("MATCH (n) DETACH DELETE n"))
        .await
        .map_err(|e| format!("Failed to wipe Neo4j: {e}"))?;

    info!("Neo4j: all nodes and relationships deleted");
    drop(graph); // release lock before setup_graph acquires it

    setup_graph().await?;
    info!("Neo4j: constraints and indexes recreated");
    Ok(())
}

/// Truncate all application tables (CASCADE handles FK dependencies).
/// Migrations are already applied by PgConnector::init, so schema is intact.
async fn do_reset_postgres() -> Result<(), DynError> {
    let pool = get_pg_pool()?;

    sqlx::query("TRUNCATE posts, watcher_cursors, places CASCADE")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to truncate PostgreSQL tables: {e}"))?;

    info!("PostgreSQL: all tables truncated");
    Ok(())
}
