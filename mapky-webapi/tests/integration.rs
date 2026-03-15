//! Integration tests for the MapKy REST API.
//!
//! These tests require Docker databases (Neo4j + PostgreSQL) to be running:
//!   cd docker && docker compose up -d
//!
//! Run with:
//!   cargo test -p mapky-webapi --test integration -- --ignored

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mapky_common::models::place::PlaceDetails;
use tower::ServiceExt;

/// Helper: GET request against the test router, return (status, body bytes).
async fn get(uri: &str) -> (StatusCode, Vec<u8>) {
    common::setup().await;
    let app = common::app();
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, body)
}

/// Helper: GET and parse JSON body as Vec<PlaceDetails>.
async fn get_places(uri: &str) -> Vec<PlaceDetails> {
    let (status, body) = get(uri).await;
    assert_eq!(status, StatusCode::OK, "body: {}", String::from_utf8_lossy(&body));
    serde_json::from_slice(&body).expect("Failed to parse places JSON")
}

// ═══════════════════════════════════════════════════════════════════
// Health
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore] // requires Docker databases
async fn test_health_returns_ok() {
    let (status, body) = get("/v0/health").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

// ═══════════════════════════════════════════════════════════════════
// Viewport — bounding box queries
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_viewport_paris_returns_three_places() {
    // Paris bounding box: Eiffel Tower, Louvre, Notre-Dame
    let places = get_places(
        "/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4",
    )
    .await;

    assert_eq!(places.len(), 3);

    let canonicals: Vec<&str> = places.iter().map(|p| p.osm_canonical.as_str()).collect();
    assert!(canonicals.contains(&"node/5765069879"), "Missing Eiffel Tower");
    assert!(canonicals.contains(&"way/53142000"), "Missing Louvre");
    assert!(canonicals.contains(&"relation/4022824"), "Missing Notre-Dame");
}

#[tokio::test]
#[ignore]
async fn test_viewport_london_returns_two_places() {
    // London bounding box: Big Ben, Buckingham Palace
    let places = get_places(
        "/v0/viewport?min_lat=51.4&min_lon=-0.2&max_lat=51.6&max_lon=0.0",
    )
    .await;

    assert_eq!(places.len(), 2);

    let canonicals: Vec<&str> = places.iter().map(|p| p.osm_canonical.as_str()).collect();
    assert!(canonicals.contains(&"node/3532563508"), "Missing Big Ben");
    assert!(canonicals.contains(&"way/4084860"), "Missing Buckingham Palace");
}

#[tokio::test]
#[ignore]
async fn test_viewport_nyc_returns_two_places() {
    // NYC bounding box: Central Park, Statue of Liberty
    let places = get_places(
        "/v0/viewport?min_lat=40.6&min_lon=-74.1&max_lat=40.8&max_lon=-73.9",
    )
    .await;

    assert_eq!(places.len(), 2);

    let canonicals: Vec<&str> = places.iter().map(|p| p.osm_canonical.as_str()).collect();
    assert!(canonicals.contains(&"relation/2552450"), "Missing Central Park");
    assert!(canonicals.contains(&"node/3456024521"), "Missing Statue of Liberty");
}

#[tokio::test]
#[ignore]
async fn test_viewport_southern_hemisphere() {
    // Sydney bounding box: Opera House
    let places = get_places(
        "/v0/viewport?min_lat=-34.0&min_lon=151.0&max_lat=-33.5&max_lon=151.5",
    )
    .await;

    assert_eq!(places.len(), 1);
    assert_eq!(places[0].osm_canonical, "way/28577776");
    assert_eq!(places[0].osm_type, "way");
    assert!((places[0].lat - (-33.8568)).abs() < 0.001);
    assert!((places[0].lon - 151.2153).abs() < 0.001);
}

#[tokio::test]
#[ignore]
async fn test_viewport_excludes_distant_places() {
    // Query Paris bbox — NYC and Sydney should NOT appear
    let places = get_places(
        "/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4",
    )
    .await;

    let canonicals: Vec<&str> = places.iter().map(|p| p.osm_canonical.as_str()).collect();

    // NYC places must not be in Paris viewport
    assert!(!canonicals.contains(&"relation/2552450"), "Central Park leaked into Paris viewport");
    assert!(!canonicals.contains(&"node/3456024521"), "Statue of Liberty leaked into Paris viewport");

    // Sydney place must not be in Paris viewport
    assert!(!canonicals.contains(&"way/28577776"), "Sydney Opera House leaked into Paris viewport");
}

#[tokio::test]
#[ignore]
async fn test_viewport_all_osm_types_in_paris() {
    // Paris has node (Eiffel Tower), way (Louvre), relation (Notre-Dame)
    let places = get_places(
        "/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4",
    )
    .await;

    let types: Vec<&str> = places.iter().map(|p| p.osm_type.as_str()).collect();
    assert!(types.contains(&"node"), "Missing node type");
    assert!(types.contains(&"way"), "Missing way type");
    assert!(types.contains(&"relation"), "Missing relation type");
}

#[tokio::test]
#[ignore]
async fn test_viewport_respects_limit() {
    // Broad European bbox covering Paris + London (5 places), but limit to 2
    let places = get_places(
        "/v0/viewport?min_lat=48.0&min_lon=-1.0&max_lat=52.0&max_lon=3.0&limit=2",
    )
    .await;

    assert_eq!(places.len(), 2);
}

#[tokio::test]
#[ignore]
async fn test_viewport_empty_area() {
    // Middle of the Pacific Ocean — no places
    let places = get_places(
        "/v0/viewport?min_lat=0.0&min_lon=-170.0&max_lat=1.0&max_lon=-169.0",
    )
    .await;

    assert!(places.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_viewport_global_returns_all_places() {
    // Global bounding box — all 8 test places
    let places = get_places(
        "/v0/viewport?min_lat=-90.0&min_lon=-180.0&max_lat=90.0&max_lon=180.0&limit=100",
    )
    .await;

    assert_eq!(places.len(), 8, "Expected all 8 test places in global viewport");
}

// ═══════════════════════════════════════════════════════════════════
// Viewport — validation (400 Bad Request)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_viewport_inverted_lat_returns_400() {
    // min_lat > max_lat
    let (status, _) = get(
        "/v0/viewport?min_lat=50.0&min_lon=2.0&max_lat=40.0&max_lon=3.0",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn test_viewport_out_of_range_lat_returns_400() {
    // lat > 90
    let (status, _) = get(
        "/v0/viewport?min_lat=0.0&min_lon=0.0&max_lat=91.0&max_lon=1.0",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn test_viewport_zero_limit_returns_400() {
    let (status, _) = get(
        "/v0/viewport?min_lat=0.0&min_lon=0.0&max_lat=1.0&max_lon=1.0&limit=0",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn test_viewport_missing_params_returns_400() {
    // Missing required parameters
    let (status, _) = get("/v0/viewport").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ═══════════════════════════════════════════════════════════════════
// Viewport — place data integrity
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_viewport_place_fields_are_complete() {
    // Query a single known place and verify all fields
    let places = get_places(
        "/v0/viewport?min_lat=-34.0&min_lon=151.0&max_lat=-33.5&max_lon=151.5",
    )
    .await;

    assert_eq!(places.len(), 1);
    let opera_house = &places[0];

    assert_eq!(opera_house.osm_canonical, "way/28577776");
    assert_eq!(opera_house.osm_type, "way");
    assert_eq!(opera_house.osm_id, 28577776);
    assert!((opera_house.lat - (-33.8568)).abs() < 0.001);
    assert!((opera_house.lon - 151.2153).abs() < 0.001);
    assert_eq!(opera_house.review_count, 0);
    assert!((opera_house.avg_rating - 0.0).abs() < f64::EPSILON);
    assert_eq!(opera_house.tag_count, 0);
    assert_eq!(opera_house.photo_count, 0);
    assert!(opera_house.indexed_at > 0);
}
