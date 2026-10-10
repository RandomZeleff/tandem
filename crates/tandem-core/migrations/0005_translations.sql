-- Translations by source text: the same English string is translated once for every
-- mod, instance and version that uses it.
CREATE TABLE translations (
    locale      TEXT NOT NULL,
    -- SHA-1 of the English text.
    source_hash TEXT NOT NULL,
    source      TEXT NOT NULL,
    target      TEXT NOT NULL,
    -- `ai` from a model, `manual` when the player corrected it (never overwritten).
    origin      TEXT NOT NULL CHECK (origin IN ('ai', 'manual')),
    model       TEXT,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (locale, source_hash)
) WITHOUT ROWID;

-- Terms the model must translate a given way. `instance_id` is '' for every instance.
CREATE TABLE glossary (
    locale      TEXT NOT NULL,
    instance_id TEXT NOT NULL DEFAULT '',
    term        TEXT NOT NULL,
    translation TEXT NOT NULL,
    PRIMARY KEY (locale, instance_id, term)
);
