-- Friends (D59): another Tandem, known by its iroh endpoint id. `state` is 'outgoing'
-- (request sent), 'incoming' (request received) or 'friend'.
CREATE TABLE friends (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    mc_uuid TEXT NOT NULL DEFAULT '',
    avatar TEXT,
    state TEXT NOT NULL,
    added_at INTEGER NOT NULL
);
