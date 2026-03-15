//! Seed the mapky-indexer databases with realistic test data.
//!
//! Populates Neo4j and PostgreSQL with real OSM locations, test users,
//! and sample posts (with ratings) so you can exercise the API without
//! needing a running Pubky homeserver.
//!
//! # Prerequisites
//!
//! Docker databases must be running:
//! ```sh
//! cd docker && docker compose up -d
//! ```
//!
//! # Usage
//!
//! ```sh
//! cargo run -p mapkyd --example seed
//! ```
//!
//! After seeding, start the API and query it:
//! ```sh
//! cargo run -p mapkyd -- api
//! # In another terminal:
//! curl 'http://localhost:8090/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4'
//! ```

use mapky_common::db::{execute_graph_operation, get_pg_pool, pg_queries, queries};
use mapky_common::models::place::PlaceDetails;
use mapky_common::models::post::PostDetails;
use mapky_common::models::user::UserDetails;
use mapky_common::types::DynError;
use mapky_common::{StackConfig, StackManager};

// ---------------------------------------------------------------------------
// Test data: real OSM locations, fictional users, sample posts
// ---------------------------------------------------------------------------

/// A place to seed, with a human-readable name for logging.
struct SeedPlace {
    name: &'static str,
    canonical: &'static str,
    osm_type: &'static str,
    osm_id: i64,
    lat: f64,
    lon: f64,
}

/// A post to seed, referencing a place and optionally a rating.
struct SeedPost {
    user_id: &'static str,
    post_id: &'static str,
    osm_canonical: &'static str,
    content: &'static str,
    rating: Option<u8>,
}

/// Real OSM locations across 4 cities, all 3 element types.
const PLACES: &[SeedPlace] = &[
    // ── Paris, France ──────────────────────────────────────────────
    SeedPlace {
        name: "Eiffel Tower",
        canonical: "node/5765069879",
        osm_type: "node",
        osm_id: 5765069879,
        lat: 48.8584,
        lon: 2.2945,
    },
    SeedPlace {
        name: "Louvre Museum",
        canonical: "way/53142000",
        osm_type: "way",
        osm_id: 53142000,
        lat: 48.8606,
        lon: 2.3376,
    },
    SeedPlace {
        name: "Notre-Dame Cathedral",
        canonical: "relation/4022824",
        osm_type: "relation",
        osm_id: 4022824,
        lat: 48.8530,
        lon: 2.3499,
    },
    // ── London, UK ─────────────────────────────────────────────────
    SeedPlace {
        name: "Big Ben",
        canonical: "node/3532563508",
        osm_type: "node",
        osm_id: 3532563508,
        lat: 51.5007,
        lon: -0.1246,
    },
    SeedPlace {
        name: "Buckingham Palace",
        canonical: "way/4084860",
        osm_type: "way",
        osm_id: 4084860,
        lat: 51.5014,
        lon: -0.1419,
    },
    // ── New York, USA ──────────────────────────────────────────────
    SeedPlace {
        name: "Central Park",
        canonical: "relation/2552450",
        osm_type: "relation",
        osm_id: 2552450,
        lat: 40.7829,
        lon: -73.9654,
    },
    SeedPlace {
        name: "Statue of Liberty",
        canonical: "node/3456024521",
        osm_type: "node",
        osm_id: 3456024521,
        lat: 40.6892,
        lon: -74.0445,
    },
    // ── Sydney, Australia (southern hemisphere) ────────────────────
    SeedPlace {
        name: "Sydney Opera House",
        canonical: "way/28577776",
        osm_type: "way",
        osm_id: 28577776,
        lat: -33.8568,
        lon: 151.2153,
    },
];

/// Two fictional test users.
const USERS: &[(&str, &str)] = &[
    ("alice_test_pk_000001", "Alice"),
    ("bob_test_pk_000002", "Bob"),
];

/// Sample posts: reviews with ratings and plain comments.
const POSTS: &[SeedPost] = &[
    // Alice's reviews
    SeedPost {
        user_id: "alice_test_pk_000001",
        post_id: "0000000000001",
        osm_canonical: "node/5765069879",
        content: "Stunning views from the top! Worth the queue.",
        rating: Some(9),
    },
    SeedPost {
        user_id: "alice_test_pk_000001",
        post_id: "0000000000002",
        osm_canonical: "way/53142000",
        content: "Could spend days here. The Mona Lisa is smaller than expected.",
        rating: Some(8),
    },
    SeedPost {
        user_id: "alice_test_pk_000001",
        post_id: "0000000000003",
        osm_canonical: "relation/2552450",
        content: "Perfect for a morning run. The Bethesda Fountain is beautiful.",
        rating: Some(10),
    },
    // Bob's reviews
    SeedPost {
        user_id: "bob_test_pk_000002",
        post_id: "0000000000004",
        osm_canonical: "node/5765069879",
        content: "Overrated but still impressive engineering.",
        rating: Some(7),
    },
    SeedPost {
        user_id: "bob_test_pk_000002",
        post_id: "0000000000005",
        osm_canonical: "relation/4022824",
        content: "The restoration work is incredible. A must-see.",
        rating: Some(9),
    },
    SeedPost {
        user_id: "bob_test_pk_000002",
        post_id: "0000000000006",
        osm_canonical: "way/28577776",
        content: "Caught a sunset concert here. Magical acoustics.",
        rating: Some(8),
    },
    // A comment without a rating
    SeedPost {
        user_id: "alice_test_pk_000001",
        post_id: "0000000000007",
        osm_canonical: "node/3532563508",
        content: "Heard the bells chime at noon — what an experience!",
        rating: None,
    },
];

// ---------------------------------------------------------------------------
// Seeding logic
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), DynError> {
    println!("Connecting to databases...");
    StackManager::setup("seed", &StackConfig::default()).await?;

    let pool = get_pg_pool()?;
    let now = chrono::Utc::now().timestamp_millis();

    // ── Users ──────────────────────────────────────────────────────
    println!("\n── Users ─────────────────────────────────────────");
    for (id, name) in USERS {
        let user = UserDetails {
            id: id.to_string(),
            name: name.to_string(),
            indexed_at: now,
        };
        let q = queries::put::create_user(&user);
        execute_graph_operation(q).await?;
        println!("  {name:20} ({id})");
    }

    // ── Places ─────────────────────────────────────────────────────
    println!("\n── Places ────────────────────────────────────────");
    for sp in PLACES {
        let place = PlaceDetails {
            osm_canonical: sp.canonical.to_string(),
            osm_type: sp.osm_type.to_string(),
            osm_id: sp.osm_id,
            lat: sp.lat,
            lon: sp.lon,
            review_count: 0,
            avg_rating: 0.0,
            tag_count: 0,
            photo_count: 0,
            indexed_at: now,
        };

        // Seed into Neo4j (spatial graph)
        let q = queries::put::create_place(&place);
        execute_graph_operation(q).await?;

        // Seed into PostgreSQL (relational + aggregates)
        pg_queries::place::upsert_place(pool, &place).await?;

        println!(
            "  {:<25} {:14} ({:.4}, {:.4})",
            sp.name, sp.canonical, sp.lat, sp.lon
        );
    }

    // ── Posts ──────────────────────────────────────────────────────
    println!("\n── Posts ─────────────────────────────────────────");
    for sp in POSTS {
        let post = PostDetails {
            id: sp.post_id.to_string(),
            author_id: sp.user_id.to_string(),
            osm_canonical: sp.osm_canonical.to_string(),
            content: Some(sp.content.to_string()),
            rating: sp.rating,
            indexed_at: now,
        };

        // Neo4j: create post with AUTHORED + ABOUT relationships
        let q = queries::put::create_post(&post);
        execute_graph_operation(q).await?;

        // PostgreSQL: insert post row
        pg_queries::post::upsert_post(pool, &post).await?;

        // PostgreSQL: update place aggregates if this is a review
        if let Some(rating) = sp.rating {
            pg_queries::place::increment_review(pool, sp.osm_canonical, rating).await?;
        }

        let rating_str = sp
            .rating
            .map(|r| format!("★ {r}/10"))
            .unwrap_or_else(|| "comment".to_string());
        let user_name = USERS
            .iter()
            .find(|(id, _)| *id == sp.user_id)
            .map(|(_, n)| *n)
            .unwrap_or("?");
        println!("  {user_name:6} → {:<14} {rating_str:10} {:.50}", sp.osm_canonical, sp.content);
    }

    // ── Summary ───────────────────────────────────────────────────
    println!("\n── Done ──────────────────────────────────────────");
    println!("  {} users, {} places, {} posts seeded.", USERS.len(), PLACES.len(), POSTS.len());
    println!();
    println!("Verify with curl (start the API first: cargo run -p mapkyd -- api):");
    println!();
    println!("  # Paris (3 places, Eiffel Tower has 2 reviews)");
    println!("  curl -s 'localhost:8090/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4' | jq .");
    println!();
    println!("  # London (2 places)");
    println!("  curl -s 'localhost:8090/v0/viewport?min_lat=51.4&min_lon=-0.2&max_lat=51.6&max_lon=0.0' | jq .");
    println!();
    println!("  # Global (all 8 places)");
    println!("  curl -s 'localhost:8090/v0/viewport?min_lat=-90&min_lon=-180&max_lat=90&max_lon=180&limit=100' | jq .");
    println!();
    println!("  # Neo4j browser: http://localhost:7474");
    println!("  # Try: MATCH (u:User)-[:AUTHORED]->(p:Post)-[:ABOUT]->(place:Place) RETURN *");

    Ok(())
}
