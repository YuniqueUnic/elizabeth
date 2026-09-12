//! 内容存储后端抽象
//!
//! `room_contents.path` 保存的是不透明的 locator 字符串，解释权在所选后端：
//! - 本地文件系统：绝对路径（与历史行为一致，存量数据无需迁移）；
//! - S3 兼容对象存储：对象 key（如 "5/report.pdf"）。
//!
//! 上传侧统一先把请求体落到本地暂存区（预留目录），再由后端把暂存文件
//! 转为正式内容（FS 用 rename 原子落位，S3 流式上传），保证失败路径
//! 不会在正式存储里留下半截内容。

use std::path::{Path, PathBuf};
use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio::io::AsyncWriteExt;

use crate::config::S3StorageConfig;

pub type ContentStream = Pin<Box<dyn Stream<Item = StorageResult<bytes::Bytes>> + Send>>;

/// 存储操作错误
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// I/O 错误
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Opendal 错误
    #[error("Storage error: {0}")]
    Opendal(#[from] opendal::Error),

    /// 内容不存在
    #[error("Content not found: {0}")]
    NotFound(String),

    /// 其他错误
    #[error("Storage error: {0}")]
    Other(String),

    /// 当前后端不支持该操作（如本地文件系统不支持预签名）
    #[error("Operation not supported by this storage backend")]
    Unsupported,
}

pub type StorageResult<T> = Result<T, StorageError>;

/// 内容存储后端
///
/// key 形如 "{room_id}/{file_name}"（已做文件名净化与冲突唯一化）；
/// locator 是持久化到 `room_contents.path` 的内容寻址串，由后端解释。
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// key 对应的正式内容是否已存在（用于上传文件名冲突检测）。
    async fn exists_key(&self, key: &str) -> StorageResult<bool>;

    /// 把本地暂存文件的内容写入 key，返回应持久化的 locator。
    async fn store_file(&self, key: &str, local: &Path) -> StorageResult<String>;

    /// 打开 locator 的内容读取流。
    async fn read(&self, locator: &str) -> StorageResult<ContentStream>;

    /// 删除 locator 对应内容；不存在视为成功。
    async fn delete(&self, locator: &str) -> StorageResult<()>;

    /// 清空房间全部存储内容（房间回收）。
    async fn purge_room(&self, room_id: i64) -> StorageResult<()>;

    /// 对象大小（字节）；presigned 提交时核对客户端直传结果。
    async fn object_size(&self, locator: &str) -> StorageResult<u64>;

    /// 签发短时效的读 URL；不支持预签名的后端返回 Unsupported。
    async fn presign_read(&self, locator: &str, ttl: std::time::Duration) -> StorageResult<String> {
        let _ = (locator, ttl);
        Err(StorageError::Unsupported)
    }

    /// 签发短时效的写 URL；不支持预签名的后端返回 Unsupported。
    async fn presign_write(&self, key: &str, ttl: std::time::Duration) -> StorageResult<String> {
        let _ = (key, ttl);
        Err(StorageError::Unsupported)
    }
}

/// 本地文件系统后端。locator = 绝对路径。
pub struct FsBackend {
    root: PathBuf,
}

impl FsBackend {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn absolute(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }
}

#[async_trait]
impl StorageBackend for FsBackend {
    async fn exists_key(&self, key: &str) -> StorageResult<bool> {
        Ok(tokio::fs::try_exists(self.absolute(key)).await?)
    }

    async fn store_file(&self, key: &str, local: &Path) -> StorageResult<String> {
        let target = self.absolute(key);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // 暂存区与正式目录同在 storage root 下，rename 原子且无跨盘问题。
        tokio::fs::rename(local, &target).await?;
        Ok(target.to_string_lossy().into_owned())
    }

    async fn read(&self, locator: &str) -> StorageResult<ContentStream> {
        let file = match tokio::fs::File::open(locator).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(StorageError::NotFound(locator.to_string()));
            }
            Err(error) => return Err(error.into()),
        };
        Ok(Box::pin(
            tokio_util::io::ReaderStream::new(file).map(|chunk| chunk.map_err(StorageError::from)),
        ))
    }

    async fn delete(&self, locator: &str) -> StorageResult<()> {
        match tokio::fs::remove_file(locator).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn purge_room(&self, room_id: i64) -> StorageResult<()> {
        match tokio::fs::remove_dir_all(self.root.join(room_id.to_string())).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn object_size(&self, locator: &str) -> StorageResult<u64> {
        match tokio::fs::metadata(locator).await {
            Ok(meta) => Ok(meta.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(locator.to_string()))
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// Opendal 后端（S3 兼容对象存储）。locator = 对象 key。
pub struct OpendalBackend {
    operator: opendal::Operator,
    presign_base_url: Option<String>,
}

impl OpendalBackend {
    /// 用给定 Operator 构造（生产走 S3，测试可注入任意 opendal 服务）。
    pub fn from_operator(operator: opendal::Operator) -> Self {
        Self {
            operator,
            presign_base_url: None,
        }
    }

    /// 设置预签名 URL 的自定义公网 base URL（CDN / 自定义域名）。
    ///
    /// 注意：S3 SigV4 会把 Host 绑入签名，替换 Host 仅在签名不绑定 Host 的
    /// 服务（如 MinIO 配置 domain、透明签名代理）或自定义域名场景下可用。
    pub fn with_presign_base_url(mut self, base_url: Option<String>) -> Self {
        self.presign_base_url = base_url;
        self
    }

    fn apply_presign_base_url(&self, url: String) -> String {
        match &self.presign_base_url {
            Some(base) => {
                let trimmed = base.trim_end_matches('/');
                match url.split_once("://") {
                    Some((_, rest)) => match rest.split_once('/') {
                        Some((_, path)) => format!("{trimmed}/{path}"),
                        None => trimmed.to_string(),
                    },
                    None => url,
                }
            }
            None => url,
        }
    }

    /// 构造 S3 兼容后端（AWS S3 / MinIO / Cloudflare R2）。
    pub fn s3(
        config: &S3StorageConfig,
        root: &str,
        presign_base_url: Option<String>,
    ) -> StorageResult<Self> {
        if config.endpoint.trim().is_empty() || config.bucket.trim().is_empty() {
            return Err(StorageError::Other(
                "storage.s3.endpoint and storage.s3.bucket are required when backend = s3"
                    .to_string(),
            ));
        }

        let builder = opendal::services::S3::default()
            .root(root)
            .endpoint(&config.endpoint)
            .bucket(&config.bucket)
            .access_key_id(&config.access_key_id)
            .secret_access_key(&config.secret_access_key);

        let builder = if let Some(region) = &config.region {
            builder.region(region)
        } else {
            builder
        };

        Ok(Self {
            operator: opendal::Operator::new(builder)?,
            presign_base_url,
        })
    }
}

#[async_trait]
impl StorageBackend for OpendalBackend {
    async fn exists_key(&self, key: &str) -> StorageResult<bool> {
        Ok(self.operator.exists(key).await?)
    }

    async fn store_file(&self, key: &str, local: &Path) -> StorageResult<String> {
        use tokio_util::compat::TokioAsyncReadCompatExt;

        let file = tokio::fs::File::open(local).await?;
        let mut writer = self.operator.writer(key).await?.into_futures_async_write();
        futures::io::copy(&mut file.compat(), &mut writer)
            .await
            .map_err(StorageError::from)?;
        futures::AsyncWriteExt::close(&mut writer)
            .await
            .map_err(StorageError::from)?;
        Ok(key.to_string())
    }

    async fn read(&self, locator: &str) -> StorageResult<ContentStream> {
        let reader = self.operator.reader(locator).await.map_err(|error| {
            if error.kind() == opendal::ErrorKind::NotFound {
                StorageError::NotFound(locator.to_string())
            } else {
                StorageError::Opendal(error)
            }
        })?;
        let stream = reader.into_bytes_stream(..).await?;
        Ok(Box::pin(
            stream.map(|chunk| chunk.map_err(StorageError::from)),
        ))
    }

    async fn delete(&self, locator: &str) -> StorageResult<()> {
        self.operator.delete(locator).await?;
        Ok(())
    }

    async fn purge_room(&self, room_id: i64) -> StorageResult<()> {
        self.operator
            .delete_with(&format!("{room_id}/"))
            .recursive(true)
            .await?;
        Ok(())
    }

    async fn object_size(&self, locator: &str) -> StorageResult<u64> {
        let metadata = self.operator.stat(locator).await.map_err(|error| {
            if error.kind() == opendal::ErrorKind::NotFound {
                StorageError::NotFound(locator.to_string())
            } else {
                StorageError::Opendal(error)
            }
        })?;
        Ok(metadata.content_length())
    }

    async fn presign_read(&self, locator: &str, ttl: std::time::Duration) -> StorageResult<String> {
        let request = self.operator.presign_read(locator, ttl).await?;
        Ok(self.apply_presign_base_url(request.uri().to_string()))
    }

    async fn presign_write(&self, key: &str, ttl: std::time::Duration) -> StorageResult<String> {
        let request = self.operator.presign_write(key, ttl).await?;
        Ok(self.apply_presign_base_url(request.uri().to_string()))
    }
}

/// 按部署配置选择存储后端。
///
/// 切换到对象存储后，新内容写入主后端；历史本地内容（locator 以 `/` 开头）
/// 仍由本机 FS 后端解释，保持可读可删，无需迁移即可继续下载。
pub fn from_config(
    config: &crate::config::StorageConfig,
) -> StorageResult<std::sync::Arc<dyn StorageBackend>> {
    let transfer = &config.transfer;
    match (&config.s3, transfer) {
        (Some(s3), _) => {
            if config.presign_ttl_seconds <= 0 {
                return Err(StorageError::Other(
                    "storage.presign_ttl_seconds must be greater than 0".to_string(),
                ));
            }
            let primary = OpendalBackend::s3(s3, "/", config.presign_base_url.clone())?;
            Ok(std::sync::Arc::new(RouterBackend::new(
                primary,
                FsBackend::new(config.root.clone()),
            )))
        }
        (None, crate::config::TransferMode::Proxy) => {
            Ok(std::sync::Arc::new(FsBackend::new(config.root.clone())))
        }
        (None, crate::config::TransferMode::Presigned) => Err(StorageError::Other(
            "storage.transfer = presigned requires storage.backend = s3".to_string(),
        )),
    }
}

/// 双后端路由：locator 以 `/` 开头 = 历史 FS 绝对路径；其余 = 主后端的 key。
pub struct RouterBackend {
    primary: OpendalBackend,
    local_fs: FsBackend,
}

impl RouterBackend {
    pub fn new(primary: OpendalBackend, local_fs: FsBackend) -> Self {
        Self { primary, local_fs }
    }
}

#[async_trait]
impl StorageBackend for RouterBackend {
    async fn exists_key(&self, key: &str) -> StorageResult<bool> {
        self.primary.exists_key(key).await
    }

    async fn store_file(&self, key: &str, local: &Path) -> StorageResult<String> {
        self.primary.store_file(key, local).await
    }

    async fn read(&self, locator: &str) -> StorageResult<ContentStream> {
        if locator.starts_with('/') {
            self.local_fs.read(locator).await
        } else {
            self.primary.read(locator).await
        }
    }

    async fn delete(&self, locator: &str) -> StorageResult<()> {
        if locator.starts_with('/') {
            self.local_fs.delete(locator).await
        } else {
            self.primary.delete(locator).await
        }
    }

    async fn purge_room(&self, room_id: i64) -> StorageResult<()> {
        self.primary.purge_room(room_id).await
    }

    async fn object_size(&self, locator: &str) -> StorageResult<u64> {
        if locator.starts_with('/') {
            self.local_fs.object_size(locator).await
        } else {
            self.primary.object_size(locator).await
        }
    }

    async fn presign_read(&self, locator: &str, ttl: std::time::Duration) -> StorageResult<String> {
        if locator.starts_with('/') {
            // 历史本地内容无法预签名，调用方（下载端点）回落到代理传输。
            return Err(StorageError::Unsupported);
        }
        self.primary.presign_read(locator, ttl).await
    }
}

/// presigned 直传专用 key：服务端生成 uuid 段，杜绝客户端互相覆盖。
pub fn presigned_key(room_id: i64, file_name: &str) -> String {
    let safe_name = sanitize_filename::sanitize(file_name);
    format!("{room_id}/{}/{safe_name}", uuid::Uuid::new_v4().simple())
}

/// 计算唯一 key：冲突时追加 (N)。文件名统一净化，杜绝路径逃逸。
pub async fn unique_key(
    backend: &dyn StorageBackend,
    room_id: i64,
    file_name: &str,
) -> StorageResult<String> {
    let safe_name = sanitize_filename::sanitize(file_name);
    let base_key = format!("{room_id}/{safe_name}");
    if !backend.exists_key(&base_key).await? {
        return Ok(base_key);
    }

    let path = Path::new(&safe_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&safe_name);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    for counter in 1..1000 {
        let candidate = if extension.is_empty() {
            format!("{room_id}/{stem}({counter})")
        } else {
            format!("{room_id}/{stem}({counter}).{extension}")
        };
        if !backend.exists_key(&candidate).await? {
            return Ok(candidate);
        }
    }

    Err(StorageError::Other(
        "Too many files with the same name".to_string(),
    ))
}
