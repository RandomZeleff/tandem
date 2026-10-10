-- Game window size at launch (NULL: the game's default) and the block drawn as the
-- instance's icon when it has no image (NULL: picked from its id).
ALTER TABLE instances ADD COLUMN window_width INTEGER;
ALTER TABLE instances ADD COLUMN window_height INTEGER;
ALTER TABLE instances ADD COLUMN block INTEGER;
