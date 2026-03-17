use crate::models::place::PlaceDetails;
use crate::types::DynError;
use sqlx::{FromRow, PgPool};

/// Row shape for place queries.
#[derive(Debug, FromRow)]
struct PlaceRow {
    pub osm_canonical: String,
    pub osm_type: String,
    pub osm_id: i64,
    pub lat: f64,
    pub lon: f64,
    pub review_count: i32,
    pub avg_rating: f64,
    pub tag_count: i32,
    pub photo_count: i32,
}

impl From<PlaceRow> for PlaceDetails {
    fn from(row: PlaceRow) -> Self {
        Self {
            osm_canonical: row.osm_canonical,
            osm_type: row.osm_type,
            osm_id: row.osm_id,
            lat: row.lat,
            lon: row.lon,
            review_count: row.review_count as i64,
            avg_rating: row.avg_rating,
            tag_count: row.tag_count as i64,
            photo_count: row.photo_count as i64,
            indexed_at: 0, // PG doesn't store indexed_at — caller can overlay from graph if needed
        }
    }
}

/// Retrieve a place by its canonical OSM reference.
pub async fn get_place_by_canonical(
    pool: &PgPool,
    osm_canonical: &str,
) -> Result<Option<PlaceDetails>, DynError> {
    let row = sqlx::query_as::<_, PlaceRow>(
        "SELECT osm_canonical, osm_type, osm_id, lat, lon,
                review_count, avg_rating, tag_count, photo_count
         FROM places WHERE osm_canonical = $1",
    )
    .bind(osm_canonical)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(PlaceDetails::from))
}

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
