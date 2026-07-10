use std::io;
use std::path::PathBuf;

pub fn reservation_dir(reservation_id: i64) -> PathBuf {
    std::env::temp_dir()
        .join("elizabeth")
        .join("chunks")
        .join(reservation_id.to_string())
}

pub fn chunk_path(reservation_id: i64, chunk_index: i64) -> PathBuf {
    reservation_dir(reservation_id).join(format!("chunk_{chunk_index}"))
}

pub fn merged_file_path(reservation_id: i64) -> PathBuf {
    reservation_dir(reservation_id).join("merged_file")
}

pub async fn remove_reservation_dir(reservation_id: i64) -> io::Result<()> {
    match tokio::fs::remove_dir_all(reservation_dir(reservation_id)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
