BEGIN IMMEDIATE;
CREATE TABLE saves (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    version INTEGER NOT NULL CHECK(version >= 0),
    state_json TEXT NOT NULL CHECK(json_valid(state_json)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE generation_attempts (
    id INTEGER PRIMARY KEY,
    save_id TEXT NOT NULL REFERENCES saves(id),
    base_turn INTEGER NOT NULL,
    attempt INTEGER NOT NULL,
    action TEXT NOT NULL,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    raw_response TEXT,
    metadata_json TEXT CHECK(metadata_json IS NULL OR json_valid(metadata_json)),
    state_before TEXT NOT NULL CHECK(json_valid(state_before)),
    status TEXT NOT NULL CHECK(status IN ('running','accepted','rejected','interrupted')),
    error TEXT,
    started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    finished_at TEXT
);
CREATE INDEX attempts_by_save ON generation_attempts(save_id,id);
CREATE TABLE turns (
    save_id TEXT NOT NULL REFERENCES saves(id),
    turn INTEGER NOT NULL CHECK(turn > 0),
    action TEXT NOT NULL,
    response_json TEXT NOT NULL CHECK(json_valid(response_json)),
    state_before TEXT NOT NULL CHECK(json_valid(state_before)),
    state_after TEXT NOT NULL CHECK(json_valid(state_after)),
    attempt_id INTEGER NOT NULL UNIQUE REFERENCES generation_attempts(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY(save_id,turn)
);
CREATE TABLE memories (
    id INTEGER PRIMARY KEY,
    save_id TEXT NOT NULL,
    turn INTEGER NOT NULL,
    text TEXT NOT NULL,
    tags_json TEXT NOT NULL CHECK(json_valid(tags_json)),
    importance REAL NOT NULL CHECK(importance >= 0 AND importance <= 1),
    FOREIGN KEY(save_id,turn) REFERENCES turns(save_id,turn)
);
CREATE INDEX memory_recency ON memories(save_id,turn DESC);
CREATE INDEX memory_importance ON memories(save_id,importance DESC,turn DESC);
PRAGMA user_version=1;
COMMIT;
