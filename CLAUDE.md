# MapKy Indexer — Geo-Spatial Social Graph Indexer

Rust service that watches Pubky homeserver events for `mapky.app/` paths, indexes data into Neo4j (spatial graph) + PostgreSQL (relational/FTS), and serves a REST API.

Architecture mirrors `pubky-nexus` (4-crate workspace) with PostgreSQL replacing Redis.

## Architecture

Cargo workspace with 4 crates:
- `mapky-common/` — Shared library: config, DB connectors, models, graph queries, types
- `mapky-watcher/` — Event listener that indexes homeserver events into DBs
- `mapky-webapi/` — REST API server (Axum, Swagger UI at /swagger-ui)
- `mapkyd/` — CLI binary: runs API, watcher, or both

### Data Stores
- **Neo4j** — Spatial graph. Place nodes have `POINT INDEX` on `location` property for viewport bbox queries via `point.withinBBox()`. Posts link to Places via `ABOUT` relationships, Users via `AUTHORED`.
- **PostgreSQL** — Relational data, aggregation caches (review_count, avg_rating), full-text search on post content. Migrations in `migrations/`.

### Crate Dependency Graph
```
mapkyd → mapky-webapi → mapky-common
       → mapky-watcher → mapky-common
```

All crates depend on `mapky-app-specs` (workspace dependency) for the type definitions.

## Commands
```sh
cargo check --workspace          # Compile check all crates
cargo test --workspace           # Run all tests (17 unit tests)
cargo clippy --workspace         # Lint
cargo run -p mapkyd              # Run both API + watcher (default)
cargo run -p mapkyd -- api       # Run API only
cargo run -p mapkyd -- watcher   # Run watcher only
cargo run -p mapkyd -- --help    # Show CLI help
```

Config file: `config/config.toml` (auto-created from defaults if missing).

## Code Patterns

### 1. OnceCell Static Connectors
DB connectors are global singletons initialized once at startup:
```rust
// mapky-common/src/db/connectors/neo4j.rs
pub static NEO4J_CONNECTOR: OnceCell<Neo4jConnector> = OnceCell::new();
pub fn get_neo4j_graph() -> Result<Arc<Mutex<Graph>>, &'static str>

// mapky-common/src/db/connectors/postgres.rs
pub static PG_CONNECTOR: OnceCell<PgPool> = OnceCell::new();
pub fn get_pg_pool() -> Result<&'static PgPool, &'static str>
```
Initialized in `StackManager::setup()` which is called by builders before starting services.

### 2. Graph Query Builders
All Neo4j queries are parameterized functions returning `neo4rs::Query`:
```rust
// mapky-common/src/db/graph/queries/put.rs
pub fn create_place(place: &PlaceDetails) -> Query { query("MERGE ...").param("key", val) }
pub fn create_post(post: &PostDetails) -> Query { ... }
pub fn create_user(user: &UserDetails) -> Query { ... }

// queries/get.rs
pub fn get_places_in_viewport(min_lat, min_lon, max_lat, max_lon, limit) -> Query
pub fn get_post_by_id(author_id, post_id) -> Query

// queries/del.rs
pub fn delete_post(author_id, post_id) -> Query
```
**NEVER use string interpolation in Cypher** — always `.param()`.

### 3. OperationOutcome
Graph mutations return a tri-state result:
```rust
pub enum OperationOutcome {
    Updated,            // node existed, was modified
    CreatedOrDeleted,   // structural change (new or removed)
    MissingDependency,  // required node not found — queue for retry
}
```
Queries must return a boolean column named `flag` for `execute_graph_operation()` to interpret.

### 4. Graph Execution Helpers
```rust
execute_graph_operation(query) -> Result<OperationOutcome>  // mutation with flag
exec_single_row(query) -> Result<()>                        // fire-and-forget
fetch_row_from_graph(query) -> Result<Option<Row>>          // single row
fetch_all_rows_from_graph(query) -> Result<Vec<Row>>        // multiple rows
fetch_key_from_graph<T>(query, key) -> Result<Option<T>>    // deserialize one column
```

### 5. Testnet Mode
The watcher supports testnet mode for local development with a pubky-docker homeserver:
- `testnet = true` in `[watcher]` config passes the testnet host to `PubkyConnector::initialise()`
- `testnet = false` (production) passes `None`, connecting to mainnet
- Default config ships with `testnet = true` for local dev
- Production: use a separate config dir via `--config-dir` with `testnet = false`

### 6. Config System
TOML-based config with trait-driven loading:
```rust
pub trait ConfigLoader<T: DeserializeOwned> {
    fn try_from_str(value: &str) -> Result<T, DynError>
    fn load(path) -> impl Future<Output = Result<T, DynError>>
}
```
Each config struct (`ApiConfig`, `WatcherConfig`, `DaemonConfig`) implements `ConfigLoader<Self>`.

`DaemonConfig` is the top-level config combining `api`, `watcher`, and shared `stack` (log_level + db).
Default config is embedded via `include_str!("../../../config/config.toml")`.

### 7. Watcher Handler Pattern
Handlers follow this flow: parse event → ensure dependencies → create detail model → put to graph → put to pg → update aggregates.
```rust
// mapky-watcher/src/events/handlers/post.rs
pub async fn sync_put(post: &MapkyAppPost, user_id: &str, post_id: &str) -> Result<(), DynError>
pub async fn del(user_id: &str, post_id: &str) -> Result<(), DynError>
```
Event dispatch in `events/mod.rs` routes `MapkyAppObject` variants to the right handler.

### 8. API Endpoints
Axum handlers with utoipa annotations for OpenAPI:
```rust
#[utoipa::path(get, path = "/v0/viewport", params(ViewportQuery), responses(...))]
pub async fn viewport(Query(params): Query<ViewportQuery>) -> Result<Json<Vec<PlaceDetails>>, Error>
```
Errors implement `IntoResponse` mapping to HTTP status codes.

### 9. Launcher Pattern
`DaemonLauncher::start()` runs API + watcher concurrently via `tokio::try_join!`, sharing a shutdown signal (`tokio::sync::watch::Receiver<bool>`) triggered by Ctrl-C.

## Key Models

### PlaceDetails (central spatial node)
```rust
pub struct PlaceDetails {
    pub osm_canonical: String,  // "node/123456" — primary key in both Neo4j and PG
    pub osm_type: String,       // "node", "way", "relation"
    pub osm_id: i64,
    pub lat: f64, pub lon: f64, // coordinates (Neo4j stores as point() for spatial index)
    pub review_count: i64, pub avg_rating: f64,  // aggregation counters
    pub tag_count: i64, pub photo_count: i64,
    pub indexed_at: i64,
}
```

### PostDetails
```rust
pub struct PostDetails {
    pub id: String,              // TimestampId from MapkyAppPost
    pub author_id: String,       // Pubky user ID
    pub osm_canonical: String,   // links to Place
    pub content: Option<String>,
    pub rating: Option<u8>,      // 1-10 if this is a review
    pub indexed_at: i64,
}
```

### UserDetails
Minimal: `id`, `name`, `indexed_at`. Extended as needed.

## Neo4j Graph Schema

### Nodes
- `(:User {id})` — Pubky user
- `(:Place {osm_canonical, osm_type, osm_id, location, lat, lon, review_count, avg_rating, ...})` — OSM element
- `(:Post {id, content, rating, indexed_at})` — review/comment/question

### Relationships
- `(:User)-[:AUTHORED]->(:Post)` — user wrote a post
- `(:Post)-[:ABOUT]->(:Place)` — post is about a place

### Indexes
- `POINT INDEX placeLocationIndex FOR (p:Place) ON (p.location)` — **core spatial index** for viewport queries
- Uniqueness constraints on User.id, Place.osm_canonical, Post.id

### Spatial Queries
Viewport queries use Neo4j's built-in spatial:
```cypher
MATCH (p:Place)
WHERE point.withinBBox(p.location,
    point({latitude: $min_lat, longitude: $min_lon}),
    point({latitude: $max_lat, longitude: $max_lon}))
RETURN p LIMIT $limit
```

## PostgreSQL Schema (migrations/)

- `places` — osm_canonical PK, coordinates, aggregation counters
- `posts` — (author_id, id) composite PK, osm_canonical FK, content with GIN FTS index
- `watcher_cursors` — cursor persistence per homeserver

## API Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/v0/health` | Version + status |
| GET | `/v0/viewport?min_lat=&min_lon=&max_lat=&max_lon=&zoom=&limit=` | Places in bounding box |
| GET | `/swagger-ui` | Interactive API docs |

## IMPORTANT Rules

- **NEVER modify mapky-app-specs types here** — changes originate in mapky-app-specs repo
- When adding a new model type, update ALL layers: watcher handler, common model + queries, webapi endpoint
- Neo4j POINT INDEX is critical for viewport performance — always store `location = point({latitude, longitude})`
- Use `osm_canonical` (e.g. "node/123") as the universal place identifier across all stores
- PostgreSQL aggregation counters (review_count, avg_rating) must stay in sync with graph mutations
- All Cypher queries MUST use `.param()` — never string interpolation
- Config defaults live in `config/config.toml` — keep in sync with config struct defaults

## What's NOT Implemented Yet (TODO)
- LocationTag, Collection, Incident, GeoCapture, Route handlers (only Post exists as reference)
- Most API endpoints (only health + viewport)
- Actual homeserver event polling (watcher loop is stubbed)
- PostgreSQL writes in handlers (graph writes work, pg writes are TODO)
- File proxy / thumbnail generation
- OSM API coordinate lookup (PlaceDetails.from_osm_ref stubs lat/lon to 0.0)
- Retry queue for MissingDependency outcomes
- Search (FTS, geocoding)
- Docker production deployment (local dev Docker setup exists in `docker/`)
