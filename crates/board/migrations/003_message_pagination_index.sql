CREATE INDEX IF NOT EXISTS idx_room_contents_room_type_sequence_id
    ON room_contents(room_id, content_type, sequence_number, id);
