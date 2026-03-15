> **Warning:** This project is in early development. Only the foundation scaffold is in place — most handlers and endpoints are not yet implemented.

# mapky-indexer

Geo-spatial indexer for [MapKy](https://mapky.app) — watches Pubky homeserver events, indexes social map data into Neo4j (spatial graph) + PostgreSQL (relational/FTS), and serves a REST API.

Architecture mirrors [pubky-nexus](https://github.com/pubky/pubky-nexus) with PostgreSQL replacing Redis.

## Workspace

| Crate | Description |
|---|---|
| `mapky-common` | Shared library — config, DB connectors, models, graph queries |
| `mapky-watcher` | Event listener that indexes homeserver events into databases |
| `mapky-webapi` | REST API server (Axum) with Swagger UI at `/swagger-ui` |
| `mapkyd` | CLI binary — runs API, watcher, or both |

## Prerequisites

- Rust (stable)
- Docker + Docker Compose (for local databases)
- [pubky-docker](https://github.com/pubky/pubky-docker) (for testnet homeserver)

## Quick Start

```sh
# 1. Start local databases
cd docker && cp .env-sample .env && docker compose up -d && cd ..

# 2. Start pubky-docker testnet homeserver (separate terminal)
# See https://github.com/pubky/pubky-docker

# 3. Run the indexer (testnet mode by default)
cargo run -p mapkyd
```

## Configuration

Default config at `config/config.toml`. Ships with `testnet = true` for local development.

```toml
[watcher]
testnet = true           # use local pubky homeserver
testnet_host = "localhost"

[stack.db.neo4j]
uri = "bolt://localhost:7687"
user = "neo4j"
password = "12345678"

[stack.db.postgres]
url = "postgres://mapky:mapky@localhost:5432/mapky"
```

For production, create a separate config directory with `testnet = false` and production credentials, then run:
```sh
cargo run -p mapkyd -- --config-dir /path/to/prod-config/
```

## Docker Services

The `docker/` directory provides Neo4j and PostgreSQL for local development:

```sh
cd docker
cp .env-sample .env       # edit credentials if needed
docker compose up -d       # start databases
docker compose down        # stop databases
docker compose down -v     # stop and delete data
```

| Service | Port | UI |
|---|---|---|
| Neo4j | 7687 (bolt) | http://localhost:7474 |
| PostgreSQL | 5432 | — |

## Usage

```sh
# Run both API and watcher (default)
cargo run -p mapkyd

# Run API only
cargo run -p mapkyd -- api

# Run watcher only
cargo run -p mapkyd -- watcher

# Show help
cargo run -p mapkyd -- --help
```

## Development

```sh
# Check compilation
cargo check --workspace

# Run unit tests (22 tests, no Docker required)
cargo test --workspace

# Run integration tests (15 tests, requires Docker databases)
cargo test -p mapky-webapi --test integration -- --ignored

# Lint
cargo clippy --workspace
```

### Integration Tests

The integration tests in `mapky-webapi/tests/integration.rs` seed real OSM locations into Neo4j and test the viewport API end-to-end:

| City | Places | OSM Types |
|---|---|---|
| Paris | Eiffel Tower, Louvre, Notre-Dame | node, way, relation |
| London | Big Ben, Buckingham Palace | node, way |
| New York | Central Park, Statue of Liberty | relation, node |
| Sydney | Opera House | way |

Tests cover: bounding box filtering, all OSM element types, southern hemisphere coordinates, limit enforcement, parameter validation (400 errors), and data integrity checks.

**Requires** `docker compose up -d` in `docker/` before running.

### Seed Script

Populate both databases with realistic test data (users, places, posts with ratings) without needing a Pubky homeserver:

```sh
# 1. Start databases
cd docker && docker compose up -d && cd ..

# 2. Seed data
cargo run -p mapkyd --example seed

# 3. Start the API
cargo run -p mapkyd -- api

# 4. Query it
curl -s 'localhost:8090/v0/viewport?min_lat=48.8&min_lon=2.2&max_lat=48.9&max_lon=2.4' | jq .
```

The seed script creates 2 users, 8 real-world OSM places (across Paris, London, NYC, Sydney), and 7 posts with ratings. It writes to both Neo4j and PostgreSQL, including aggregate updates (review_count, avg_rating).

After seeding, you can also explore the graph visually at http://localhost:7474 with:
```cypher
MATCH (u:User)-[:AUTHORED]->(p:Post)-[:ABOUT]->(place:Place) RETURN *
```

### Full End-to-End Testing (with Pubky Homeserver)

To test the complete pipeline including the watcher event polling:

```sh
# 1. Start local databases
cd docker && docker compose up -d && cd ..

# 2. Start pubky-docker testnet homeserver (see https://github.com/pubky/pubky-docker)
# This provides the homeserver at the ID configured in config.toml

# 3. Run the full daemon (API + watcher)
cargo run -p mapkyd

# 4. Write data to the homeserver using the Pubky SDK
#    The watcher polls every 5s and indexes new events into Neo4j + PostgreSQL

# 5. Query the API to verify indexed data
curl -s 'localhost:8090/v0/health' | jq .
curl -s 'localhost:8090/v0/viewport?min_lat=-90&min_lon=-180&max_lat=90&max_lon=180&limit=100' | jq .
```

## License

MIT
