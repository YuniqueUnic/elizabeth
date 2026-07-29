use std::io;
use std::path::{Path, PathBuf};

/// Keep chunk staging on the configured storage filesystem so completing an
/// upload can atomically rename the verified file into its room directory.
pub fn staging_root(storage_root: &Path) -> PathBuf {
    storage_root.join(".chunks")
}

pub fn reservation_dir(storage_root: &Path, reservation_id: i64) -> PathBuf {
    staging_root(storage_root).join(reservation_id.to_string())
}

pub fn chunk_path(storage_root: &Path, reservation_id: i64, chunk_index: i64) -> PathBuf {
    reservation_dir(storage_root, reservation_id).join(format!("chunk_{chunk_index}"))
}

pub fn merged_file_path(storage_root: &Path, reservation_id: i64) -> PathBuf {
    reservation_dir(storage_root, reservation_id).join("merged_file")
}

pub async fn remove_reservation_dir(storage_root: &Path, reservation_id: i64) -> io::Result<()> {
    match tokio::fs::remove_dir_all(reservation_dir(storage_root, reservation_id)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
