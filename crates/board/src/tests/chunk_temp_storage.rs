use std::path::Path;

#[test]
fn chunk_staging_paths_are_scoped_to_the_configured_storage_root() {
    let storage_root = Path::new("/srv/elizabeth/storage/rooms");
    let reservation_id = 42;
    let reservation_dir = crate::chunk_temp_storage::reservation_dir(storage_root, reservation_id);

    assert_eq!(
        crate::chunk_temp_storage::staging_root(storage_root),
        storage_root.join(".chunks")
    );
    assert_eq!(reservation_dir, storage_root.join(".chunks/42"));
    assert_eq!(
        crate::chunk_temp_storage::chunk_path(storage_root, reservation_id, 3),
        reservation_dir.join("chunk_3")
    );
    assert_eq!(
        crate::chunk_temp_storage::merged_file_path(storage_root, reservation_id),
        reservation_dir.join("merged_file")
    );
}
