-- Empty-schema bootstrap: exactly the V1 tables plus the V3..V5 additions.
-- Column order, defaults and indices must match the historical upgrade path.
-- All other tables, indices and triggers still come from the migration runner.
CREATE TABLE episodes (
    id TEXT PRIMARY KEY,
    continuity_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    valid_from_ms INTEGER,
    valid_until_ms INTEGER,
    created_ms INTEGER,
    provenance TEXT
);
CREATE INDEX idx_episodes_created_ms ON episodes(created_ms);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    closed_at INTEGER,
    title TEXT,
    scope TEXT,
    project_id TEXT,
    state TEXT,
    metadata_json TEXT,
    revision INTEGER NOT NULL DEFAULT 0,
    archived_at INTEGER,
    updated_at INTEGER
);
CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_sessions_scope ON sessions(scope);

CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    content TEXT NOT NULL,
    source_episode_ids_json TEXT NOT NULL DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0.5,
    tags_json TEXT NOT NULL DEFAULT '[]',
    valid_from INTEGER,
    valid_until INTEGER
);
