-- Key/value launcher settings; values are JSON.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Account metadata only. Tokens live in the OS credential store, never here (see DECISIONS D9).
CREATE TABLE accounts (
    id         TEXT PRIMARY KEY NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('microsoft', 'offline')),
    username   TEXT NOT NULL,
    mc_uuid    TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- At most one active account.
CREATE UNIQUE INDEX accounts_single_active ON accounts (is_active) WHERE is_active = 1;

CREATE TABLE instances (
    id             TEXT PRIMARY KEY NOT NULL,
    name           TEXT NOT NULL,
    game_version   TEXT NOT NULL,
    loader         TEXT NOT NULL DEFAULT 'vanilla'
                   CHECK (loader IN ('vanilla', 'fabric', 'quilt', 'forge', 'neoforge')),
    loader_version TEXT,
    java_path      TEXT,
    memory_mb      INTEGER,
    jvm_args       TEXT,
    icon           TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_played_at TEXT
);
