use crate::models::place::PlaceDetails;
use crate::types::DynError;
use sqlx::PgPool;

/// Upsert a place row. On conflict, update coordinates (preserves aggregates).
pub async fn upsert_place(pool: &PgPool, place: &PlaceDetails) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO places (osm_canonical, osm_type, osm_id, lat, lon)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (osm_canonical)
         DO UPDATE SET lat = $4, lon = $5, updated_at = NOW()",
    )
    .bind(&place.osm_canonical)
    .bind(&place.osm_type)
    .bind(place.osm_id)
    .bind(place.lat)
    .bind(place.lon)
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment review_count and recalculate avg_rating using running average.
pub async fn increment_review(
    pool: &PgPool,
    osm_canonical: &str,
    rating: u8,
) -> Result<(), DynError> {
    sqlx::query(
        "UPDATE places
         SET review_count = review_count + 1,
             avg_rating = ((avg_rating * review_count) + $2) / (review_count + 1),
             updated_at = NOW()
         WHERE osm_canonical = $1",
    )
    .bind(osm_canonical)
    .bind(f64::from(rating))
    .execute(pool)
    .await?;
    Ok(())
}

/// Decrement review_count and reverse the running average.
/// Resets avg_rating to 0 if count reaches 0.
pub async fn decrement_review(
    pool: &PgPool,
    osm_canonical: &str,
    rating: u8,
) -> Result<(), DynError> {
    sqlx::query(
        "UPDATE places
         SET review_count = GREATEST(review_count - 1, 0),
             avg_rating = CASE
                 WHEN review_count <= 1 THEN 0.0
                 ELSE ((avg_rating * review_count) - $2) / (review_count - 1)
             END,
             updated_at = NOW()
         WHERE osm_canonical = $1",
    )
    .bind(osm_canonical)
    .bind(f64::from(rating))
    .execute(pool)
    .await?;
    Ok(())
}
