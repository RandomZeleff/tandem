-- Modrinth modpack an instance was created from (NULL for regular instances and
-- for packs imported from a file).
ALTER TABLE instances ADD COLUMN pack_project_id TEXT;
ALTER TABLE instances ADD COLUMN pack_version_id TEXT;
ALTER TABLE instances ADD COLUMN pack_version TEXT;
