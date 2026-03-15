# MapKy Indexer — Task Runner
# Run `just` to see all available recipes.

# List all recipes
default:
    @just --list

# Start Docker databases (Neo4j + PostgreSQL)
up:
    cd docker && docker compose up -d

# Stop Docker databases
down:
    cd docker && docker compose down

# Follow Docker database logs
logs:
    cd docker && docker compose logs -f

# Run full daemon (API + watcher) with local config
dev: up
    cargo run -p mapkyd

# Run full daemon with testnet config (requires pubky-docker)
dev-testnet: up
    cargo run -p mapkyd -- --config-dir config/testnet

# Run API only
api:
    cargo run -p mapkyd -- api

# Run watcher only
watcher:
    cargo run -p mapkyd -- watcher

# Seed databases with test data (users, places, posts)
seed: up
    cargo run -p mapkyd --example seed

# Write test posts to testnet homeserver (requires pubky-docker)
write-testnet:
    cargo run -p mapkyd --example write_testnet

# Wipe both databases and recreate schema
reset: up
    cargo run -p mapkyd -- reset-db

# Reset + seed (clean slate with test data)
fresh: reset seed

# Run unit tests (no Docker required)
test:
    cargo test --workspace

# Run integration tests (requires Docker databases + seed data)
test-int: up seed
    cargo test -p mapky-webapi --test integration -- --ignored

# Run clippy + format check
check:
    cargo clippy --workspace
    cargo fmt --check

# Format all code
fmt:
    cargo fmt
