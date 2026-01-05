//! TypeScript 类型导出模块
//!
//! 此模块集中导出所有需要生成 TypeScript 类型定义的 Rust 类型。
//! 使用 ts-rs 库自动生成前端 TypeScript 类型。

#[cfg(feature = "typescript-export")]
pub mod export;

// 重新导出所有 API 相关类型，方便前端使用
pub use crate::models::content::{ContentType, RoomContent};
pub use crate::models::room::{Room, RoomStatus, RoomToken};
// 注意：RoomPermission 是 bitflags，不支持 TS derive，需要手动在前端定义
pub use crate::models::room::chunk_upload::{
    ChunkStatus, ChunkUploadRequest, ChunkUploadResponse, RoomChunkUpload,
};
pub use crate::models::room::refresh_token::{
    CreateRefreshTokenRequest, RefreshTokenRequest, RefreshTokenResponse, RoomRefreshToken,
    TokenBlacklistEntry,
};
pub use crate::models::room::upload_reservation::{
    RoomUploadReservation, UploadFileDescriptor, UploadStatus,
};
