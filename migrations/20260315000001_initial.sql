-- MapKy initial schema

-- Places: aggregated place data, keyed by OSM canonical reference
CREATE TABLE IF NOT EXISTS places (
    osm_canonical TEXT PRIMARY KEY,       -- "node/123456789"
    osm_type TEXT NOT NULL,               -- "node", "way", "relation"
    osm_id BIGINT NOT NULL,
    lat DOUBLE PRECISION NOT NULL,
    lon DOUBLE PRECISION NOT NULL,
    review_count INTEGER NOT NULL DEFAULT 0,
    avg_rating DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    tag_count INTEGER NOT NULL DEFAULT 0,
    photo_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Posts: reviews, questions, comments
CREATE TABLE IF NOT EXISTS posts (
    author_id TEXT NOT NULL,
    id TEXT NOT NULL,
    osm_canonical TEXT NOT NULL REFERENCES places(osm_canonical),
    content TEXT,
    rating SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (author_id, id)
);

-- Full-text search index on post content
CREATE INDEX IF NOT EXISTS posts_content_fts ON posts USING GIN (to_tsvector('english', COALESCE(content, '')));

-- Spatial reference index for place lookups
CREATE INDEX IF NOT EXISTS places_osm_type_id ON places (osm_type, osm_id);

-- Watcher cursor persistence
CREATE TABLE IF NOT EXISTS watcher_cursors (
    homeserver_id TEXT PRIMARY KEY,
    cursor TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
