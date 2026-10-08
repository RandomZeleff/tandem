-- Disabled content keeps its file, renamed `<file_name>.disabled`.
ALTER TABLE instance_content
    ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1));
