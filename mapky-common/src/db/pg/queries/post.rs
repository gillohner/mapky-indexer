use crate::models::post::PostDetails;
use crate::types::DynError;
use sqlx::{FromRow, PgPool};

/// Row shape for post queries.
#[derive(Debug, FromRow)]
struct PostRow {
    pub author_id: String,
    pub id: String,
    pub osm_canonical: String,
    pub content: Option<String>,
    pub rating: Option<i16>,
}

impl From<PostRow> for PostDetails {
    fn from(row: PostRow) -> Self {
        Self {
            id: row.id,
            author_id: row.author_id,
            osm_canonical: row.osm_canonical,
            content: row.content,
            rating: row.rating.map(|r| r as u8),
            indexed_at: 0, // PG doesn't store indexed_at
        }
    }
}

/// Retrieve posts for a place, newest first.
/// If `reviews_only` is true, only posts with a rating are returned.
pub async fn get_posts_for_place(
    pool: &PgPool,
    osm_canonical: &str,
    reviews_only: bool,
    skip: i64,
    limit: i64,
) -> Result<Vec<PostDetails>, DynError> {
    let rows = if reviews_only {
        sqlx::query_as::<_, PostRow>(
            "SELECT author_id, id, osm_canonical, content, rating
             FROM posts
             WHERE osm_canonical = $1 AND rating IS NOT NULL
             ORDER BY created_at DESC
             OFFSET $2 LIMIT $3",
        )
        .bind(osm_canonical)
        .bind(skip)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, PostRow>(
            "SELECT author_id, id, osm_canonical, content, rating
             FROM posts
             WHERE osm_canonical = $1
             ORDER BY created_at DESC
             OFFSET $2 LIMIT $3",
        )
        .bind(osm_canonical)
        .bind(skip)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows.into_iter().map(PostDetails::from).collect())
}

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
