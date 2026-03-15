use mapky_common::db::{execute_graph_operation, get_neo4j_graph, queries};
use mapky_common::models::place::PlaceDetails;
use mapky_common::{StackConfig, StackManager};
use neo4rs::query;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

/// Initialize DB connections and seed test data.
/// Safe to call multiple times — only executes once per process.
pub async fn setup() {
    SETUP
        .get_or_init(|| async {
            StackManager::setup("mapky-test", &StackConfig::default())
                .await
                .expect("Failed to connect to test databases — is Docker running?");

            cleanup_graph().await;
            seed_places().await;
        })
        .await;
}

/// Build the API router for testing (no server needed).
pub fn app() -> axum::Router {
    mapky_webapi::routes::routes()
}

// ---------------------------------------------------------------------------
// Test Data: Real OSM locations across 4 cities, all 3 OSM element types
// ---------------------------------------------------------------------------

struct TestPlace {
    canonical: &'static str,
    osm_type: &'static str,
    osm_id: i64,
    lat: f64,
    lon: f64,
}

const TEST_PLACES: &[TestPlace] = &[
    // ── Paris, France ──────────────────────────────────────────────
    TestPlace {
        canonical: "node/5765069879",
        osm_type: "node",
        osm_id: 5765069879,
        lat: 48.8584,
        lon: 2.2945,
    }, // Eiffel Tower
    TestPlace {
        canonical: "way/53142000",
        osm_type: "way",
        osm_id: 53142000,
        lat: 48.8606,
        lon: 2.3376,
    }, // Louvre Museum
    TestPlace {
        canonical: "relation/4022824",
        osm_type: "relation",
        osm_id: 4022824,
        lat: 48.8530,
        lon: 2.3499,
    }, // Notre-Dame
    // ── London, UK ─────────────────────────────────────────────────
    TestPlace {
        canonical: "node/3532563508",
        osm_type: "node",
        osm_id: 3532563508,
        lat: 51.5007,
        lon: -0.1246,
    }, // Big Ben
    TestPlace {
        canonical: "way/4084860",
        osm_type: "way",
        osm_id: 4084860,
        lat: 51.5014,
        lon: -0.1419,
    }, // Buckingham Palace
    // ── New York, USA ──────────────────────────────────────────────
    TestPlace {
        canonical: "relation/2552450",
        osm_type: "relation",
        osm_id: 2552450,
        lat: 40.7829,
        lon: -73.9654,
    }, // Central Park
    TestPlace {
        canonical: "node/3456024521",
        osm_type: "node",
        osm_id: 3456024521,
        lat: 40.6892,
        lon: -74.0445,
    }, // Statue of Liberty
    // ── Sydney, Australia (southern hemisphere) ────────────────────
    TestPlace {
        canonical: "way/28577776",
        osm_type: "way",
        osm_id: 28577776,
        lat: -33.8568,
        lon: 151.2153,
    }, // Sydney Opera House
];

async fn cleanup_graph() {
    let graph = get_neo4j_graph().expect("Neo4j not initialized");
    let graph = graph.lock().await;
    let mut result = graph
        .execute(query("MATCH (n) DETACH DELETE n"))
        .await
        .expect("Failed to clean graph");
    while result.next().await.unwrap().is_some() {}
}

async fn seed_places() {
    for tp in TEST_PLACES {
        let place = PlaceDetails {
            osm_canonical: tp.canonical.to_string(),
            osm_type: tp.osm_type.to_string(),
            osm_id: tp.osm_id,
            lat: tp.lat,
            lon: tp.lon,
            review_count: 0,
            avg_rating: 0.0,
            tag_count: 0,
            photo_count: 0,
            indexed_at: chrono::Utc::now().timestamp_millis(),
        };
        let q = queries::put::create_place(&place);
        execute_graph_operation(q)
            .await
            .unwrap_or_else(|e| panic!("Failed to seed place {}: {e}", tp.canonical));
    }
}
