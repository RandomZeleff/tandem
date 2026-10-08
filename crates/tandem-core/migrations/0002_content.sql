-- Mods, resource packs and shaders installed in an instance (one row per project).
CREATE TABLE instance_content (
    instance_id    TEXT NOT NULL REFERENCES instances (id) ON DELETE CASCADE,
    project_id     TEXT NOT NULL,
    version_id     TEXT NOT NULL,
    kind           TEXT NOT NULL CHECK (kind IN ('mod', 'resourcepack', 'shader')),
    title          TEXT NOT NULL,
    version_number TEXT NOT NULL,
    file_name      TEXT NOT NULL,
    sha1           TEXT NOT NULL,
    icon_url       TEXT,
    -- Pulled in automatically as a required dependency of another project.
    is_dependency  INTEGER NOT NULL DEFAULT 0 CHECK (is_dependency IN (0, 1)),
    installed_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (instance_id, project_id)
);
