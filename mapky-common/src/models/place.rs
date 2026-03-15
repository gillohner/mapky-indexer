use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use mapky_app_specs::OsmRef;

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
    /// Create a PlaceDetails from an OsmRef.
    /// Coordinates are stubbed to 0.0 — a real implementation would
    /// look up coordinates via the OSM API or Nominatim.
    pub fn from_osm_ref(osm_ref: &OsmRef) -> Self {
        Self {
            osm_canonical: osm_ref.canonical(),
            osm_type: osm_ref.osm_type.to_string(),
            osm_id: osm_ref.osm_id,
            // TODO: Look up real coordinates via OSM API / Nominatim
            lat: 0.0,
            lon: 0.0,
            review_count: 0,
            avg_rating: 0.0,
            tag_count: 0,
            photo_count: 0,
            indexed_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapky_app_specs::OsmElementType;

    #[test]
    fn test_place_details_from_osm_ref() {
        let osm_ref = OsmRef::new(OsmElementType::Node, 1573053883);
        let place = PlaceDetails::from_osm_ref(&osm_ref);
        assert_eq!(place.osm_canonical, "node/1573053883");
        assert_eq!(place.osm_type, "node");
        assert_eq!(place.osm_id, 1573053883);
        assert_eq!(place.review_count, 0);
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
