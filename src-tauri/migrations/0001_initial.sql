-- AstroHour SQLite schema
-- Migration 0001: initial

-- Per-hour notes.
-- planet_hour_key = "YYYY-MM-DD:N" where N is the hour index (0-23)
-- within the day schedule (not clock hour).
CREATE TABLE IF NOT EXISTS notes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    planet_key   TEXT    NOT NULL,   -- e.g. "2025-07-04:3"
    body         TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notes_planet_key ON notes (planet_key);

-- Full-text search virtual table over note bodies.
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts
    USING fts5(body, content='notes', content_rowid='id');

-- Keep FTS in sync with notes.
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts (rowid, body) VALUES (new.id, new.body);
END;
CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts (notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
END;
CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts (notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
    INSERT INTO notes_fts (rowid, body) VALUES (new.id, new.body);
END;

-- License state (single row, always id=1).
-- key_hash: SHA-256 hex of the license key (for display / re-entry detection).
-- activated_at: ISO-8601 datetime of first activation.
-- The actual key is stored encrypted in tauri-plugin-stronghold, not here.
CREATE TABLE IF NOT EXISTS license (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    key_hash     TEXT,
    activated_at TEXT
);

-- Seed the single license row so UPDATEs always work.
INSERT OR IGNORE INTO license (id) VALUES (1);
