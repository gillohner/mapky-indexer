//! Write test data to a pubky-docker testnet homeserver.
//!
//! Creates ephemeral users, signs them up on the testnet homeserver,
//! and PUTs MapkyAppPost blobs to `/pub/mapky.app/posts/` paths.
//! The watcher (running with testnet config) will pick these up
//! and index them into Neo4j + PostgreSQL.
//!
//! # Prerequisites
//!
//! 1. pubky-docker testnet running (homeserver + postgres + pkarr)
//!    ```sh
//!    ./mapky-dev start --testnet
//!    ```
//!
//! 2. Wait for the daemon window to show "Listening on 0.0.0.0:8090"
//!
//! # Usage
//!
//! ```sh
//! # In a separate terminal:
//! cargo run -p mapkyd --example write_testnet
//!
//! # Wait ~10s for watcher poll + Nominatim lookups, then verify:
//! curl -s 'localhost:8090/v0/viewport?min_lat=-90&min_lon=-180&max_lat=90&max_lon=180&limit=100' | jq .
//! curl -s 'localhost:8090/v0/place/node/5765069879' | jq .
//! curl -s 'localhost:8090/v0/place/node/5765069879/posts' | jq .
//! ```

use mapky_app_specs::traits::{HasIdPath, TimestampId};
use mapky_app_specs::{MapkyAppPost, OsmElementType, OsmRef};
use pubky::{Keypair, PubkyHttpClient, PublicKey};

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

/// Create a testnet SDK client and sign up a user.
/// Tolerates Pkarr DHT publish failures (common with isolated testnet DHT).
async fn signup_user(
    homeserver: &PublicKey,
    user_index: usize,
) -> Result<(String, pubky::PubkySession), Box<dyn std::error::Error + Send + Sync>> {
    let client = PubkyHttpClient::testnet()?;
    let pubky = pubky::Pubky::with_client(client);

    let keypair = Keypair::random();
    let pk = keypair.public_key().to_z32();
    let signer = pubky.signer(keypair);

    match signer.signup(homeserver, None).await {
        Ok(session) => {
            println!("  User {}: {pk}", user_index + 1);
            Ok((pk, session))
        }
        Err(e) => {
            let err_str = format!("{e}");
            // The signup HTTP request succeeds but Pkarr DHT publish fails
            // on isolated testnet. Try signin instead — the account exists.
            if err_str.contains("NoClosestNodes") || err_str.contains("Pkarr") {
                println!(
                    "  User {}: {pk} (signup ok, DHT publish skipped — isolated testnet)",
                    user_index + 1
                );
                let session = signer.signin().await?;
                Ok((pk, session))
            } else {
                Err(e.into())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Connecting to pubky-docker testnet...\n");

    let homeserver = PublicKey::try_from(HOMESERVER_PK)?;

    // Create two test users
    let user_count = 2;
    let mut sessions = Vec::new();

    println!("── Creating {user_count} test users ──────────────────────");
    for i in 0..user_count {
        let (pk, session) = signup_user(&homeserver, i).await?;
        sessions.push((pk, session));
    }

    let posts = test_posts();

    // Write posts, alternating between users
    println!(
        "\n── Writing {} posts to homeserver ────────────────",
        posts.len()
    );
    for (i, (place, content, rating)) in posts.iter().enumerate() {
        let (ref user_pk, ref session) = sessions[i % sessions.len()];

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

        println!("  [{status}] {:.12}… → {path}  ({rating_str})", user_pk);

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
    println!("  curl -s 'localhost:8090/v0/place/node/5765069879' | jq .");
    println!("  curl -s 'localhost:8090/v0/place/node/5765069879/posts' | jq .");
    println!();

    // Print user public keys for debugging
    println!("  User public keys (for events-stream debugging):");
    for (i, (pk, _)) in sessions.iter().enumerate() {
        println!("    User {}: {pk}", i + 1);
    }

    Ok(())
}
