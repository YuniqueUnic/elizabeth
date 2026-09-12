use tempfile::TempDir;

use crate::config::S3StorageConfig;
use crate::storage::{
    FsBackend, OpendalBackend, RouterBackend, StorageBackend, from_config, unique_key,
};

async fn write_local(path: &std::path::Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    tokio::fs::write(path, contents).await.unwrap();
}

fn assert_locator_keys(locator: &str) {
    // Opendal 后端的 locator 是对象 key，不允许绝对路径。
    assert!(
        !locator.starts_with('/'),
        "locator must be a key: {locator}"
    );
}

#[tokio::test]
async fn fs_backend_stores_reads_and_purges() {
    let root = TempDir::new().unwrap();
    let backend = FsBackend::new(root.path().to_path_buf());

    let local = root.path().join("staging.bin");
    write_local(&local, b"payload").await;

    let locator = backend.store_file("7/data.bin", &local).await.unwrap();
    assert!(locator.starts_with(root.path().to_str().unwrap()));
    assert!(!local.exists(), "FS 后端用 rename 转正，暂存文件应消失");
    assert!(backend.exists_key("7/data.bin").await.unwrap());

    use futures::StreamExt;
    let stream = backend.read(&locator).await.unwrap();
    let bytes: Vec<u8> = stream
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();
    assert_eq!(bytes, b"payload");

    backend.delete(&locator).await.unwrap();
    assert!(!backend.exists_key("7/data.bin").await.unwrap());
    // 删除不存在的内容视为成功
    backend.delete(&locator).await.unwrap();
}

#[tokio::test]
async fn fs_backend_purge_room_removes_directory() {
    let root = TempDir::new().unwrap();
    let backend = FsBackend::new(root.path().to_path_buf());

    let local = root.path().join("x.bin");
    write_local(&local, b"x").await;
    backend.store_file("3/x.bin", &local).await.unwrap();

    backend.purge_room(3).await.unwrap();
    assert!(!root.path().join("3").exists());
    // 房间不存在时 purge 也应成功
    backend.purge_room(999).await.unwrap();
}

#[tokio::test]
async fn opendal_backend_uses_object_keys_as_locators() {
    let root = TempDir::new().unwrap();
    let operator = opendal::Operator::new(
        opendal::services::Fs::default().root(root.path().to_str().unwrap()),
    )
    .unwrap();
    let backend = OpendalBackend::from_operator(operator);

    let local = root.path().join("staging.txt");
    write_local(&local, b"opendal payload").await;

    let locator = backend.store_file("11/notes.txt", &local).await.unwrap();
    assert_locator_keys(&locator);

    use futures::StreamExt;
    let stream = backend.read(&locator).await.unwrap();
    let bytes: Vec<u8> = stream
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();
    assert_eq!(bytes, b"opendal payload");

    backend.purge_room(11).await.unwrap();
    assert!(!backend.exists_key("11/notes.txt").await.unwrap());
}

#[tokio::test]
async fn unique_key_sanitizes_names_and_avoids_collisions() {
    let root = TempDir::new().unwrap();
    let backend = FsBackend::new(root.path().to_path_buf());

    // 路径穿越形状的名字被净化为安全 key：不含分隔符、不指向父目录
    let key = unique_key(&backend, 5, "../../etc/passwd").await.unwrap();
    let name = key.strip_prefix("5/").unwrap();
    assert!(!name.contains('/') && !name.contains('\\') && name != "..");

    // 冲突时追加 (N)
    let local = root.path().join("tmp.bin");
    write_local(&local, b"1").await;
    let first = unique_key(&backend, 5, "report.pdf").await.unwrap();
    backend.store_file(&first, &local).await.unwrap();
    let second = unique_key(&backend, 5, "report.pdf").await.unwrap();
    assert_eq!(second, "5/report(1).pdf");
}

#[tokio::test]
async fn from_config_selects_backend_by_s3_presence() {
    let root = TempDir::new().unwrap();

    let fs_config = crate::config::StorageConfig {
        root: root.path().to_path_buf(),
        upload_reservation_ttl_seconds: 600,
        s3: None,
        transfer: crate::config::TransferMode::Proxy,
        presign_base_url: None,
        presign_ttl_seconds: 300,
    };
    assert!(from_config(&fs_config).is_ok());

    let s3_config = crate::config::StorageConfig {
        s3: Some(S3StorageConfig {
            endpoint: "https://s3.example.com".to_string(),
            bucket: "elizabeth".to_string(),
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(), // pragma: allowlist secret
            region: Some("auto".to_string()),
        }),
        ..fs_config.clone()
    };
    // 只验证构造成功；不触网，不发起真实 S3 请求
    assert!(from_config(&s3_config).is_ok());
}

#[tokio::test]
async fn from_config_rejects_incomplete_s3_settings() {
    let root = TempDir::new().unwrap();
    let config = crate::config::StorageConfig {
        root: root.path().to_path_buf(),
        upload_reservation_ttl_seconds: 600,
        s3: Some(S3StorageConfig {
            endpoint: String::new(),
            bucket: String::new(),
            access_key_id: String::new(),
            secret_access_key: String::new(), // pragma: allowlist secret
            region: None,
        }),
        transfer: crate::config::TransferMode::Proxy,
        presign_base_url: None,
        presign_ttl_seconds: 300,
    };
    assert!(from_config(&config).is_err());
}

#[tokio::test]
async fn router_backend_serves_legacy_fs_locators_and_new_keys() {
    let fs_root = TempDir::new().unwrap();
    let op_root = TempDir::new().unwrap();
    let primary = OpendalBackend::from_operator(
        opendal::Operator::new(
            opendal::services::Fs::default().root(op_root.path().to_str().unwrap()),
        )
        .unwrap(),
    );
    let backend = RouterBackend::new(primary, FsBackend::new(fs_root.path().to_path_buf()));

    // 历史 FS locator（绝对路径）仍可读
    let legacy_path = fs_root.path().join("9/old.bin");
    write_local(&legacy_path, b"legacy").await;
    use futures::StreamExt;
    let stream = backend.read(legacy_path.to_str().unwrap()).await.unwrap();
    let bytes: Vec<u8> = stream
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();
    assert_eq!(bytes, b"legacy");

    // 新 key 走主后端
    let local = op_root.path().join("staging.bin");
    write_local(&local, b"new").await;
    let locator = backend.store_file("9/new.bin", &local).await.unwrap();
    assert!(!locator.starts_with('/'));
    let stream = backend.read(&locator).await.unwrap();
    let bytes: Vec<u8> = stream
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();
    assert_eq!(bytes, b"new");
}

#[tokio::test]
async fn from_config_rejects_presigned_transfer_without_s3_backend() {
    let root = TempDir::new().unwrap();
    let config = crate::config::StorageConfig {
        root: root.path().to_path_buf(),
        upload_reservation_ttl_seconds: 600,
        s3: None,
        transfer: crate::config::TransferMode::Presigned,
        presign_base_url: None,
        presign_ttl_seconds: 300,
    };
    assert!(from_config(&config).is_err());
}

#[tokio::test]
async fn opendal_backend_presigns_offline_and_honors_base_url() {
    let s3 = S3StorageConfig {
        endpoint: "https://s3.test".to_string(),
        bucket: "elizabeth".to_string(),
        access_key_id: "key".to_string(),
        secret_access_key: "secret".to_string(), // pragma: allowlist secret
        region: Some("auto".to_string()),
    };

    // 预签名在本地完成签名，不发起网络请求
    let backend = OpendalBackend::s3(&s3, "/", None).unwrap();
    let url = backend
        .presign_write("5/report.pdf", std::time::Duration::from_secs(300))
        .await
        .unwrap();
    assert!(url.starts_with("https://s3.test/"));
    assert!(url.contains("X-Amz-Expires=300"), "url: {url}");

    let read_url = backend
        .presign_read("5/report.pdf", std::time::Duration::from_secs(120))
        .await
        .unwrap();
    assert!(read_url.starts_with("https://s3.test/"));
    assert!(read_url.contains("X-Amz-Expires=120"), "url: {read_url}");

    // 自定义公网 base URL 替换 scheme+host，保留签名参数
    let cdn = OpendalBackend::s3(&s3, "/", Some("https://cdn.example.com".to_string())).unwrap();
    let url = cdn
        .presign_read("5/report.pdf", std::time::Duration::from_secs(300))
        .await
        .unwrap();
    assert!(url.starts_with("https://cdn.example.com/"), "url: {url}");
    assert!(url.contains("X-Amz-Expires=300"), "url: {url}");
}
