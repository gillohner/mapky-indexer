-- Rename homeserver_id to source_id in watcher_cursors.
-- The column now stores either a user public key (events-stream polling)
-- or a homeserver ID (legacy fallback polling).
ALTER TABLE watcher_cursors RENAME COLUMN homeserver_id TO source_id;
