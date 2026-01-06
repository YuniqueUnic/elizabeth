//! TypeScript 类型生成模块
//!
//! 此模块为所有 API 相关类型实现 TS trait，用于自动生成 TypeScript 类型定义。
//!
//! 注意：ts-rs 会通过 #[ts(export)] 属性自动生成类型文件。
//! 类型文件会在编译时生成到 web/types/generated/ 目录。

/// 导出函数：生成所有类型的 TypeScript 定义
#[cfg(feature = "typescript-export")]
pub fn export_types() -> String {
    // ts-rs 会自动处理所有带有 #[derive(TS)] 和 #[ts(export)] 的类型
    // 这个函数主要用于文档说明
    format!("TypeScript types exported via ts-rs")
}
#[cfg(test)]
#[cfg(feature = "typescript-export")]
mod ts_export_tests {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_all_types() {
        use crate::models::content::{ContentType, RoomContent};
        use crate::models::room::chunk_upload::{
            ChunkStatus, ChunkUploadRequest, ChunkUploadResponse, RoomChunkUpload,
        };
        use crate::models::room::refresh_token::{
            CreateRefreshTokenRequest, RefreshTokenRequest, RefreshTokenResponse, RoomRefreshToken,
            TokenBlacklistEntry,
        };
        use crate::models::room::upload_reservation::{
            RoomUploadReservation, UploadFileDescriptor, UploadStatus,
        };
        use crate::models::room::{Room, RoomStatus};

        // 导出所有类型到 TypeScript
        Room::export();
        RoomStatus::export();
        ContentType::export();
        RoomContent::export();
        ChunkStatus::export();
        RoomChunkUpload::export();
        ChunkUploadRequest::export();
        ChunkUploadResponse::export();
        CreateRefreshTokenRequest::export();
        RefreshTokenRequest::export();
        RefreshTokenResponse::export();
        RoomRefreshToken::export();
        TokenBlacklistEntry::export();
        RoomUploadReservation::export();
        UploadFileDescriptor::export();
        UploadStatus::export();
    }
}
