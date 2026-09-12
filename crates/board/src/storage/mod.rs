//! 内容存储抽象层
//!
//! 统一本地文件系统与 S3 兼容对象存储（AWS S3 / MinIO / Cloudflare R2）。
//! `room_contents.path` 保存后端解释的不透明 locator：以 `/` 开头的是历史
//! FS 绝对路径（永远由本地后端解释，保证存量可读），其余是当前主后端的
//! 对象 key。上传先落本地暂存区，再经后端原子转正。

pub mod backend;

pub use backend::{
    ContentStream, FsBackend, OpendalBackend, RouterBackend, StorageBackend, StorageError,
    StorageResult, from_config, presigned_key, unique_key,
};
