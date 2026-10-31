CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    language TEXT NOT NULL CHECK (language IN ('ar', 'en')),
    theme TEXT NOT NULL CHECK (theme IN ('light', 'dark', 'system')),
    download_dir TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
