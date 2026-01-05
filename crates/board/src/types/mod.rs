//! TypeScript 类型导出模块
//!
//! 此模块集中导出所有需要生成 TypeScript 类型定义的 Rust 类型。
//! 使用 ts-rs 库自动生成前端 TypeScript 类型。

#[cfg(feature = "typescript-export")]
pub mod export;

// 重新导出所有 API 相关类型，方便前端使用
pub use crate::models::{
    ChunkStatus, ChunkUploadRequest, ChunkUploadResponse, ContentType, CreateRefreshTokenRequest,
    RefreshTokenRequest, RefreshTokenResponse, Room, RoomChunkUpload, RoomContent, RoomPermission,
    RoomRefreshToken, RoomStatus, RoomToken, UploadFileDescriptor, UploadStatus,
};
