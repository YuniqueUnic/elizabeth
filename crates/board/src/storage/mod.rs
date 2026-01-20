//! Storage abstraction layer
//!
//! This module provides a unified storage interface that supports multiple storage backends:
//! - Local filesystem (FS)
//! - S3-compatible object storage (AWS S3, MinIO, Cloudflare R2)
//!
//! # Example
//!
//! ```rust,no_run
//! use board::storage::{OpendalBackend, StorageBackend, StorageConfig, StorageType};
//!
//! # async fn example() -> board::storage::StorageResult<()> {
//! let config = StorageConfig {
//!     storage_type: StorageType::Fs,
//!     root: "/tmp/storage".to_string(),
//!     s3_config: None,
//! };
//!
//! let backend = OpendalBackend::new(config)?;
//! backend.put("test.txt", b"Hello".to_vec()).await?;
//! # Ok(())
//! # }
//! ```

pub mod backend;

pub use backend::{
    OpendalBackend, S3Config, StorageBackend, StorageConfig, StorageError, StorageResult,
    StorageType,
};
