use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use mapky_app_specs::OsmRef;

use crate::db::connectors::nominatim;

/// Indexed representation of a place (OSM element) with aggregated social data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlaceDetails {
    pub osm_canonical: String,
    pub osm_type: String,
    pub osm_id: i64,
    pub lat: f64,
    pub lon: f64,
    pub review_count: i64,
    pub avg_rating: f64,
    pub tag_count: i64,
    pub photo_count: i64,
    pub indexed_at: i64,
}

impl PlaceDetails {
    /// Create a new PlaceDetails with explicit coordinates.
    /// Use this for tests and seeding where coordinates are already known.
    pub fn new(osm_ref: &OsmRef, lat: f64, lon: f64) -> Self {
        Self {
            osm_canonical: osm_ref.canonical(),
            osm_type: osm_ref.osm_type.to_string(),
            osm_id: osm_ref.osm_id,
            lat,
            lon,
            review_count: 0,
            avg_rating: 0.0,
            tag_count: 0,
            photo_count: 0,
            indexed_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create a PlaceDetails from an OsmRef, resolving coordinates via Nominatim.
    /// Falls back to (0.0, 0.0) if the lookup fails or Nominatim is not initialized.
    pub async fn from_osm_ref(osm_ref: &OsmRef) -> Self {
        let osm_type = osm_ref.osm_type.to_string();
        let (lat, lon) = match nominatim::resolve_osm_coords(&osm_type, osm_ref.osm_id).await {
            Ok(Some(coords)) => coords,
            Ok(None) => (0.0, 0.0),
            Err(e) => {
                tracing::warn!(
                    "Nominatim lookup failed for {}/{}: {e}",
                    osm_type,
                    osm_ref.osm_id
                );
                (0.0, 0.0)
            }
        };

        Self::new(osm_ref, lat, lon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapky_app_specs::OsmElementType;

    #[test]
    fn test_place_details_new() {
        let osm_ref = OsmRef::new(OsmElementType::Node, 1573053883);
        let place = PlaceDetails::new(&osm_ref, 48.8584, 2.2945);
        assert_eq!(place.osm_canonical, "node/1573053883");
        assert_eq!(place.osm_type, "node");
        assert_eq!(place.osm_id, 1573053883);
        assert!((place.lat - 48.8584).abs() < f64::EPSILON);
        assert!((place.lon - 2.2945).abs() < f64::EPSILON);
        assert_eq!(place.review_count, 0);
    }

    #[tokio::test]
    async fn test_from_osm_ref_without_nominatim() {
        // Without NominatimClient initialized, falls back to (0, 0)
        let osm_ref = OsmRef::new(OsmElementType::Node, 1573053883);
        let place = PlaceDetails::from_osm_ref(&osm_ref).await;
        assert_eq!(place.osm_canonical, "node/1573053883");
        assert_eq!(place.lat, 0.0);
        assert_eq!(place.lon, 0.0);
    }

    #[test]
    fn test_place_details_serde_roundtrip() {
        let place = PlaceDetails {
            osm_canonical: "way/42".to_string(),
            osm_type: "way".to_string(),
            osm_id: 42,
            lat: 51.5074,
            lon: -0.1278,
            review_count: 5,
            avg_rating: 4.2,
            tag_count: 3,
            photo_count: 10,
            indexed_at: 1000,
        };
        let json = serde_json::to_string(&place).unwrap();
        let parsed: PlaceDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.osm_canonical, place.osm_canonical);
        assert_eq!(parsed.lat, place.lat);
        assert_eq!(parsed.avg_rating, place.avg_rating);
    }
}
