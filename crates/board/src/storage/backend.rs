//! Storage backend abstraction
//!
//! This module defines a unified storage interface that supports multiple storage backends:
//! - Local filesystem (FS)
//! - S3-compatible object storage (AWS S3, MinIO, Cloudflare R2)
//!
//! # Architecture
//!
//! The storage layer is designed with the following principles:
//! - **Abstraction**: Single `StorageBackend` trait for all storage operations
//! - **Flexibility**: Easy to add new storage backends
//! - **Type Safety**: Rust's type system ensures correct usage
//! - **Async-first**: All operations are async for better performance

use async_trait::async_trait;
use opendal::{Builder, Operator};
use std::path::Path;

/// Storage backend type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    /// Local filesystem storage
    Fs,
    /// S3-compatible object storage (AWS S3, MinIO, R2)
    S3,
}

/// Storage backend configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Storage type
    pub storage_type: StorageType,

    /// Root path for file storage
    /// - For FS: local directory path
    /// - For S3: bucket prefix/path
    pub root: String,

    /// S3-specific configuration (only used when storage_type is S3)
    pub s3_config: Option<S3Config>,
}

/// S3-compatible storage configuration
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3 endpoint URL (e.g., "https://s3.amazonaws.com")
    pub endpoint: String,

    /// Bucket name
    pub bucket: String,

    /// AWS access key ID
    pub access_key_id: String,

    /// AWS secret access key
    pub secret_access_key: String,

    /// AWS region (e.g., "us-east-1")
    pub region: Option<String>,
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Opendal error occurred
    #[error("Storage error: {0}")]
    Opendal(#[from] opendal::Error),

    /// File not found
    #[error("File not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Other error
    #[error("Storage error: {0}")]
    Other(String),
}

/// Storage backend trait
///
/// This trait defines the interface for all storage backends.
/// All operations are async and return `StorageResult`.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Get a file from storage
    ///
    /// # Arguments
    /// * `path` - Relative path to the file
    ///
    /// # Returns
    /// File contents as bytes
    async fn get(&self, path: &str) -> StorageResult<Vec<u8>>;

    /// Put a file to storage
    ///
    /// # Arguments
    /// * `path` - Relative path where to store the file
    /// * `data` - File contents as bytes
    async fn put(&self, path: &str, data: Vec<u8>) -> StorageResult<()>;

    /// Delete a file from storage
    ///
    /// # Arguments
    /// * `path` - Relative path to the file to delete
    async fn delete(&self, path: &str) -> StorageResult<()>;

    /// List files in a directory
    ///
    /// # Arguments
    /// * `path` - Relative path to the directory
    ///
    /// # Returns
    /// List of file paths in the directory
    async fn list(&self, path: &str) -> StorageResult<Vec<String>>;

    /// Check if a file exists
    ///
    /// # Arguments
    /// * `path` - Relative path to the file
    ///
    /// # Returns
    /// `true` if file exists, `false` otherwise
    async fn exists(&self, path: &str) -> StorageResult<bool>;
}

/// Opendal-based storage backend implementation
///
/// This implementation uses Opendal as the underlying storage engine,
/// providing support for multiple storage services through a unified API.
pub struct OpendalBackend {
    /// Opendal operator
    operator: Operator,
}

impl OpendalBackend {
    /// Create a new Opendal backend from configuration
    ///
    /// # Arguments
    /// * `config` - Storage configuration
    ///
    /// # Returns
    /// A new `OpendalBackend` instance
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        let operator = match config.storage_type {
            StorageType::Fs => {
                // Build filesystem operator
                let builder = opendal::services::Fs::default().root(&config.root);

                Operator::new(builder).map_err(StorageError::from)?.finish()
            }
            StorageType::S3 => {
                // Build S3 operator
                let s3_cfg = config.s3_config.ok_or_else(|| {
                    StorageError::Other("S3 configuration is required for S3 storage".to_string())
                })?;

                let mut builder = opendal::services::S3::default()
                    .root(&config.root)
                    .endpoint(&s3_cfg.endpoint)
                    .bucket(&s3_cfg.bucket)
                    .access_key_id(&s3_cfg.access_key_id)
                    .secret_access_key(&s3_cfg.secret_access_key);

                if let Some(region) = s3_cfg.region {
                    builder = builder.region(&region);
                }

                Operator::new(builder).map_err(StorageError::from)?.finish()
            }
        };

        Ok(Self { operator })
    }

    /// Get the underlying Opendal operator
    pub fn operator(&self) -> &Operator {
        &self.operator
    }
}

#[async_trait]
impl StorageBackend for OpendalBackend {
    async fn get(&self, path: &str) -> StorageResult<Vec<u8>> {
        let bytes = self
            .operator
            .read(path)
            .await
            .map_err(StorageError::from)?
            .to_vec();
        Ok(bytes)
    }

    async fn put(&self, path: &str, data: Vec<u8>) -> StorageResult<()> {
        self.operator
            .write(path, data)
            .await
            .map(|_| ())
            .map_err(StorageError::from)
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        self.operator.delete(path).await.map_err(StorageError::from)
    }

    async fn list(&self, path: &str) -> StorageResult<Vec<String>> {
        let entries = self.operator.list(path).await.map_err(StorageError::from)?;

        let files = entries
            .into_iter()
            .map(|entry| entry.path().to_string())
            .collect();

        Ok(files)
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        self.operator.exists(path).await.map_err(StorageError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_fs_storage() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_str().unwrap().to_string();

        let config = StorageConfig {
            storage_type: StorageType::Fs,
            root,
            s3_config: None,
        };

        let backend = OpendalBackend::new(config).unwrap();

        // Test put
        backend
            .put("test.txt", b"Hello, World!".to_vec())
            .await
            .unwrap();

        // Test exists
        assert!(backend.exists("test.txt").await.unwrap());

        // Test get
        let data = backend.get("test.txt").await.unwrap();
        assert_eq!(data, b"Hello, World!");

        // Test list
        let files = backend.list("").await.unwrap();
        assert!(files.contains(&"test.txt".to_string()));

        // Test delete
        backend.delete("test.txt").await.unwrap();
        assert!(!backend.exists("test.txt").await.unwrap());
    }
}
