-- 0005_create_room_upload_reservations.sql
-- 上传预留表

CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    token_jti TEXT NOT NULL,
    file_manifest TEXT NOT NULL,
    reserved_size INTEGER NOT NULL,
    reserved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    consumed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

CREATE TRIGGER IF NOT EXISTS trg_room_upload_reservations_updated_at
AFTER UPDATE ON room_upload_reservations
FOR EACH ROW
BEGIN
    UPDATE room_upload_reservations
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;
