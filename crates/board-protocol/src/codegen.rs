use std::path::Path;

#[cfg(feature = "typescript-export")]
use ts_rs::TS;

#[cfg(feature = "typescript-export")]
use crate::models::content::{ContentType, RoomContent};
#[cfg(feature = "typescript-export")]
use crate::models::{
    ChunkStatus, ChunkUploadRequest, ChunkUploadResponse, CreateRefreshTokenRequest,
    RefreshTokenRequest, RefreshTokenResponse, Room, RoomChunkUpload, RoomRefreshToken, RoomStatus,
    RoomUploadReservation, TokenBlacklistEntry, UploadFileDescriptor, UploadStatus,
};

#[cfg(feature = "typescript-export")]
pub fn export_ts_types_to(output_dir: &Path) -> Result<(), ts_rs::ExportError> {
    Room::export_all_to(output_dir)?;
    RoomStatus::export_all_to(output_dir)?;
    RoomContent::export_all_to(output_dir)?;
    ContentType::export_all_to(output_dir)?;
    ChunkStatus::export_all_to(output_dir)?;
    RoomChunkUpload::export_all_to(output_dir)?;
    ChunkUploadRequest::export_all_to(output_dir)?;
    ChunkUploadResponse::export_all_to(output_dir)?;
    CreateRefreshTokenRequest::export_all_to(output_dir)?;
    RefreshTokenRequest::export_all_to(output_dir)?;
    RefreshTokenResponse::export_all_to(output_dir)?;
    RoomRefreshToken::export_all_to(output_dir)?;
    TokenBlacklistEntry::export_all_to(output_dir)?;
    RoomUploadReservation::export_all_to(output_dir)?;
    UploadFileDescriptor::export_all_to(output_dir)?;
    UploadStatus::export_all_to(output_dir)?;
    Ok(())
}

#[cfg(feature = "typescript-export")]
pub fn exported_ts_type_names() -> &'static [&'static str] {
    &[
        "Room",
        "RoomStatus",
        "RoomContent",
        "ContentType",
        "ChunkStatus",
        "RoomChunkUpload",
        "ChunkUploadRequest",
        "ChunkUploadResponse",
        "CreateRefreshTokenRequest",
        "RefreshTokenRequest",
        "RefreshTokenResponse",
        "RoomRefreshToken",
        "TokenBlacklistEntry",
        "RoomUploadReservation",
        "UploadFileDescriptor",
        "UploadStatus",
    ]
}

#[cfg(feature = "typescript-export")]
pub fn api_schema_json_pretty() -> Result<String, serde_json::Error> {
    use schemars::schema_for;

    #[derive(schemars::JsonSchema)]
    struct ApiSchema {
        room: Room,
        room_status: RoomStatus,
        room_content: RoomContent,
        content_type: ContentType,
        chunk_status: ChunkStatus,
        room_chunk_upload: RoomChunkUpload,
        chunk_upload_request: ChunkUploadRequest,
        chunk_upload_response: ChunkUploadResponse,
        create_refresh_token_request: CreateRefreshTokenRequest,
        refresh_token_request: RefreshTokenRequest,
        refresh_token_response: RefreshTokenResponse,
        room_refresh_token: RoomRefreshToken,
        token_blacklist_entry: TokenBlacklistEntry,
        room_upload_reservation: RoomUploadReservation,
        upload_file_descriptor: UploadFileDescriptor,
        upload_status: UploadStatus,
    }

    let root = schema_for!(ApiSchema);
    serde_json::to_string_pretty(&root)
}
