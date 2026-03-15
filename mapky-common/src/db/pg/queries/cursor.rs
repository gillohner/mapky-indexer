use crate::types::DynError;
use sqlx::PgPool;

/// Get the current cursor for a source (user ID or homeserver ID), or None if never polled.
pub async fn get_cursor(pool: &PgPool, source_id: &str) -> Result<Option<String>, DynError> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT cursor FROM watcher_cursors WHERE source_id = $1",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Upsert the cursor for a source (user ID or homeserver ID).
pub async fn upsert_cursor(
    pool: &PgPool,
    source_id: &str,
    cursor: &str,
) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO watcher_cursors (source_id, cursor, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (source_id)
         DO UPDATE SET cursor = $2, updated_at = NOW()",
    )
    .bind(source_id)
    .bind(cursor)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get cursors for multiple users in a single query. Returns (source_id, cursor) pairs.
pub async fn get_cursors_for_users(
    pool: &PgPool,
    user_ids: &[String],
) -> Result<Vec<(String, String)>, DynError> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT source_id, cursor FROM watcher_cursors WHERE source_id = ANY($1)",
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
