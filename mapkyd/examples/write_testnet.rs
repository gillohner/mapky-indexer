//! Write test data to a pubky-docker testnet homeserver.
//!
//! Creates ephemeral users, signs them up on the testnet homeserver,
//! and PUTs MapkyAppPost blobs to `/pub/mapky.app/posts/` paths.
//! The watcher (running with testnet config) will pick these up
//! and index them into Neo4j + PostgreSQL.
//!
//! # Prerequisites
//!
//! 1. pubky-docker testnet running (`localhost:6881` DHT, `localhost:15411` relay)
//!    See: https://github.com/pubky/pubky-docker
//!
//! 2. mapky-indexer databases running:
//!    ```sh
//!    just up
//!    ```
//!
//! # Usage
//!
//! ```sh
//! # Terminal 1: run the daemon in testnet mode
//! just dev-testnet
//!
//! # Terminal 2: write test data
//! cargo run -p mapkyd --example write_testnet
//!
//! # Terminal 2: verify indexed data (wait a few seconds for watcher poll)
//! curl -s 'localhost:8090/v0/viewport?min_lat=-90&min_lon=-180&max_lat=90&max_lon=180&limit=100' | jq .
//! ```

use mapky_app_specs::traits::{HasIdPath, TimestampId};
use mapky_app_specs::{MapkyAppPost, OsmElementType, OsmRef};
use pubky::{Keypair, PublicKey, Pubky};

/// The homeserver public key from config.toml.
/// This must match the pubky-docker instance you're running.
const HOMESERVER_PK: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

fn test_posts() -> Vec<(OsmRef, &'static str, Option<u8>)> {
    vec![
        // Paris — Eiffel Tower
        (
            OsmRef::new(OsmElementType::Node, 5765069879),
            "The Eiffel Tower at sunset is absolutely magical!",
            Some(9),
        ),
        // Paris — Louvre
        (
            OsmRef::new(OsmElementType::Way, 53142000),
            "The Louvre is overwhelming — plan for at least a full day.",
            Some(8),
        ),
        // London — Big Ben
        (
            OsmRef::new(OsmElementType::Node, 3532563508),
            "Big Ben is under renovation but still impressive from the outside.",
            Some(7),
        ),
        // New York — Central Park
        (
            OsmRef::new(OsmElementType::Relation, 2552450),
            "Central Park in spring is pure joy. Bring a picnic!",
            Some(10),
        ),
        // Sydney — Opera House
        (
            OsmRef::new(OsmElementType::Way, 28577776),
            "Sydney Opera House — the architecture is even more stunning in person.",
            Some(9),
        ),
        // Comment without rating (Eiffel Tower)
        (
            OsmRef::new(OsmElementType::Node, 5765069879),
            "Does anyone know if there's wheelchair access to the top?",
            None,
        ),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Connecting to pubky-docker testnet...\n");

    // Create testnet SDK (connects to localhost DHT + relay)
    let pubky = Pubky::testnet()?;
    let homeserver = PublicKey::try_from(HOMESERVER_PK)?;

    // Create two test users
    let user_count = 2;
    let mut sessions = Vec::new();

    println!("── Creating {user_count} test users ──────────────────────");
    for i in 0..user_count {
        let keypair = Keypair::random();
        let pk = keypair.public_key().to_z32();
        let signer = pubky.signer(keypair);

        let session = signer.signup(&homeserver, None).await?;
        println!("  User {}: {pk}", i + 1);
        sessions.push(session);
    }

    let posts = test_posts();

    // Write posts, alternating between users
    println!(
        "\n── Writing {} posts to homeserver ────────────────",
        posts.len()
    );
    for (i, (place, content, rating)) in posts.iter().enumerate() {
        let session = &sessions[i % sessions.len()];
        let user_pk = session.info().public_key().to_z32();

        // Create the MapkyAppPost and generate its ID + path
        let post = MapkyAppPost::new(
            place.clone(),
            Some(content.to_string()),
            *rating,
            None,
            None,
        );
        let post_id = post.create_id();
        let path = MapkyAppPost::create_path(&post_id);

        // Serialize to JSON and PUT to homeserver
        let body = serde_json::to_vec(&post)?;
        let response = session.storage().put(&path, body).await?;

        let status = response.status();
        let rating_str = rating
            .map(|r| format!("★ {r}/10"))
            .unwrap_or_else(|| "comment".to_string());

        println!("  [{status}] {user_pk:.12}… → {path}  ({rating_str})",);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            eprintln!("    ERROR: {body}");
        }
    }

    // Summary
    println!("\n── Done ──────────────────────────────────────────");
    println!(
        "  {} users, {} posts written to homeserver",
        user_count,
        posts.len()
    );
    println!();
    println!("  The watcher will index these on its next poll cycle.");
    println!("  Default poll interval: 5 seconds.");
    println!();
    println!("  Verify with:");
    println!("  curl -s 'localhost:8090/v0/viewport?min_lat=-90&min_lon=-180&max_lat=90&max_lon=180&limit=100' | jq .");
    println!();

    // Print user public keys for debugging
    println!("  User public keys (for events-stream debugging):");
    for (i, session) in sessions.iter().enumerate() {
        println!(
            "    User {}: {}",
            i + 1,
            session.info().public_key().to_z32()
        );
    }

    Ok(())
}
