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

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace
```

## License

MIT
