use crate::db::get_neo4j_graph;
use crate::types::DynError;
use neo4rs::{Query, Row};
use serde::de::DeserializeOwned;

/// Represents the outcome of a mutation-like query in the graph database.
#[derive(Debug)]
pub enum OperationOutcome {
    /// The query found and updated an existing node/relationship.
    Updated,
    /// A structural mutation: node/relationship was created or deleted.
    CreatedOrDeleted,
    /// A required node/relationship was not found (missing dependency).
    MissingDependency,
}

/// Executes a graph query expected to return one row with a boolean "flag" column.
///
/// - `true` => Updated
/// - `false` => CreatedOrDeleted
/// - No rows => MissingDependency
pub async fn execute_graph_operation(query: Query) -> Result<OperationOutcome, DynError> {
    let maybe_flag = fetch_key_from_graph(query, "flag").await?;
    match maybe_flag {
        Some(true) => Ok(OperationOutcome::Updated),
        Some(false) => Ok(OperationOutcome::CreatedOrDeleted),
        None => Ok(OperationOutcome::MissingDependency),
    }
}

/// Execute a graph query without reading results.
pub async fn exec_single_row(query: Query) -> Result<(), DynError> {
    let graph = get_neo4j_graph()?;
    let graph = graph.lock().await;
    let mut result = graph.execute(query).await?;
    result.next().await?;
    Ok(())
}

pub async fn fetch_row_from_graph(query: Query) -> Result<Option<Row>, DynError> {
    let graph = get_neo4j_graph()?;
    let graph = graph.lock().await;
    let mut result = graph.execute(query).await?;
    result.next().await.map_err(Into::into)
}

pub async fn fetch_all_rows_from_graph(query: Query) -> Result<Vec<Row>, DynError> {
    let graph = get_neo4j_graph()?;
    let graph = graph.lock().await;
    let mut result = graph.execute(query).await?;
    let mut rows = Vec::new();
    while let Some(row) = result.next().await? {
        rows.push(row);
    }
    Ok(rows)
}

/// Fetch the value of type T mapped to a specific key from the first row.
pub async fn fetch_key_from_graph<T>(query: Query, key: &str) -> Result<Option<T>, DynError>
where
    T: DeserializeOwned + Send + Sync,
{
    let maybe_row = fetch_row_from_graph(query).await?;
    let Some(row) = maybe_row else {
        return Ok(None);
    };
    row.get(key)
        .map(Some)
        .map_err(Into::into)
        .inspect_err(|e| tracing::error!("Failed to get {key} from query result: {e}"))
}
