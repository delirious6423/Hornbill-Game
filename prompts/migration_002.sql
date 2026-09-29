BEGIN IMMEDIATE;
CREATE TABLE image_attempts (
    id INTEGER PRIMARY KEY,
    save_id TEXT NOT NULL REFERENCES saves(id),
    turn INTEGER NOT NULL,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','succeeded','failed','interrupted')),
    result_json TEXT,
    metadata_json TEXT,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    finished_at TEXT
);
CREATE INDEX image_attempt_save_turn ON image_attempts(save_id,turn,id);
CREATE TABLE scene_images (
    save_id TEXT NOT NULL REFERENCES saves(id),
    turn INTEGER NOT NULL,
    attempt_id INTEGER NOT NULL REFERENCES image_attempts(id),
    reason TEXT NOT NULL,
    PRIMARY KEY(save_id,turn)
);
CREATE TABLE save_preferences (
    save_id TEXT PRIMARY KEY REFERENCES saves(id),
    settings_json TEXT NOT NULL
);
PRAGMA user_version=2;
COMMIT;
