use crate::types::DynError;
use sqlx::PgPool;

/// Get the current cursor for a homeserver, or None if never polled.
pub async fn get_cursor(pool: &PgPool, homeserver_id: &str) -> Result<Option<String>, DynError> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT cursor FROM watcher_cursors WHERE homeserver_id = $1",
    )
    .bind(homeserver_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Upsert the cursor for a homeserver.
pub async fn upsert_cursor(
    pool: &PgPool,
    homeserver_id: &str,
    cursor: &str,
) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO watcher_cursors (homeserver_id, cursor, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (homeserver_id)
         DO UPDATE SET cursor = $2, updated_at = NOW()",
    )
    .bind(homeserver_id)
    .bind(cursor)
    .execute(pool)
    .await?;
    Ok(())
}
