use crate::models::post::PostDetails;
use crate::types::DynError;
use sqlx::PgPool;

/// Upsert a post into PostgreSQL. On conflict, update content and rating.
pub async fn upsert_post(pool: &PgPool, post: &PostDetails) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO posts (author_id, id, osm_canonical, content, rating)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (author_id, id)
         DO UPDATE SET content = $4, rating = $5",
    )
    .bind(&post.author_id)
    .bind(&post.id)
    .bind(&post.osm_canonical)
    .bind(&post.content)
    .bind(post.rating.map(|r| r as i16))
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a post, returning (osm_canonical, rating) if it existed.
/// Both values are needed to roll back place aggregates.
pub async fn delete_post(
    pool: &PgPool,
    author_id: &str,
    post_id: &str,
) -> Result<Option<(String, Option<i16>)>, DynError> {
    let row = sqlx::query_as::<_, (String, Option<i16>)>(
        "DELETE FROM posts WHERE author_id = $1 AND id = $2 RETURNING osm_canonical, rating",
    )
    .bind(author_id)
    .bind(post_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
