use crate::db::get_neo4j_graph;
use crate::types::DynError;
use neo4rs::query;
use tracing::info;

/// Ensure the Neo4j graph has the required constraints and indexes for MapKy.
pub async fn setup_graph() -> Result<(), DynError> {
    let constraints = [
        "CREATE CONSTRAINT uniqueUserId IF NOT EXISTS FOR (u:User) REQUIRE u.id IS UNIQUE",
        "CREATE CONSTRAINT uniquePlaceId IF NOT EXISTS FOR (p:Place) REQUIRE p.osm_canonical IS UNIQUE",
        "CREATE CONSTRAINT uniquePostId IF NOT EXISTS FOR (p:Post) REQUIRE p.id IS UNIQUE",
    ];

    let indexes = [
        "CREATE INDEX userIdIndex IF NOT EXISTS FOR (u:User) ON (u.id)",
        "CREATE INDEX placeCanonicalIndex IF NOT EXISTS FOR (p:Place) ON (p.osm_canonical)",
        "CREATE INDEX postIdIndex IF NOT EXISTS FOR (p:Post) ON (p.id)",
        "CREATE INDEX postTimestampIndex IF NOT EXISTS FOR (p:Post) ON (p.indexed_at)",
        // Core spatial index for viewport queries
        "CREATE POINT INDEX placeLocationIndex IF NOT EXISTS FOR (p:Place) ON (p.location)",
    ];

    let all_queries = constraints.iter().chain(indexes.iter());

    let graph = get_neo4j_graph()?;
    let graph = graph.lock().await;

    let txn = graph
        .start_txn()
        .await
        .map_err(|e| format!("Failed to start transaction: {e}"))?;

    for &ddl in all_queries {
        if let Err(err) = graph.run(query(ddl)).await {
            return Err(format!("Failed to apply graph constraints/indexes: {err}").into());
        }
    }

    txn.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    info!("Neo4j graph constraints and indexes applied successfully");

    Ok(())
}
