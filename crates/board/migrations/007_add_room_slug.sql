ALTER TABLE rooms ADD COLUMN slug TEXT NOT NULL DEFAULT '';

UPDATE rooms
SET slug = name
WHERE slug = '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_rooms_slug ON rooms(slug);
