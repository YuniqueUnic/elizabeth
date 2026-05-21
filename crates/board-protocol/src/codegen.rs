use std::path::Path;

#[cfg(feature = "typescript-export")]
use ts_rs::TS;

#[cfg(feature = "typescript-export")]
use crate::dto::{
    ChunkStatusInfo, ChunkUploadRequest, ChunkUploadResponse, ChunkedUploadPreparationRequest,
    ChunkedUploadPreparationResponse, CleanupResponse, CreateMessageRequest, CreateMessageResponse,
    DeleteContentRequest, DeleteContentResponse, DeleteRoomResponse, FileMergeRequest,
    FileMergeResponse, FullRoomGcStatusView, IssueTokenRequest, IssueTokenResponse, LogoutRequest,
    MergedFileInfo, ReservedFileInfo, RevokeTokenResponse, RoomContentView, RoomTokenClaims,
    RoomTokenView, RunRoomGcResponse, TokenType, UpdateContentRequest, UpdateContentResponse,
    UpdateRoomPermissionRequest, UpdateRoomSettingsRequest, UploadContentResponse,
    UploadPreparationRequest, UploadPreparationResponse, UploadStatusQuery, UploadStatusResponse,
    ValidateTokenRequest, ValidateTokenResponse,
};
#[cfg(feature = "typescript-export")]
use crate::models::content::{ContentType, RoomContent};
#[cfg(feature = "typescript-export")]
use crate::models::{
    ChunkStatus, CreateRefreshTokenRequest, RefreshTokenRequest, RefreshTokenResponse, Room,
    RoomChunkUpload, RoomRefreshToken, RoomStatus, RoomUploadReservation, TokenBlacklistEntry,
    UploadFileDescriptor, UploadStatus,
};

#[cfg(feature = "typescript-export")]
pub fn export_ts_types_to(output_dir: &Path) -> Result<(), ts_rs::ExportError> {
    let output_dir_cfg = ts_rs::Config::new().with_out_dir(&output_dir);
    Room::export_all(&&output_dir_cfg)?;
    RoomStatus::export_all(&output_dir_cfg)?;
    RoomContent::export_all(&output_dir_cfg)?;
    ContentType::export_all(&output_dir_cfg)?;
    ChunkStatus::export_all(&output_dir_cfg)?;
    RoomChunkUpload::export_all(&output_dir_cfg)?;
    CreateRefreshTokenRequest::export_all(&output_dir_cfg)?;
    RefreshTokenRequest::export_all(&output_dir_cfg)?;
    RefreshTokenResponse::export_all(&output_dir_cfg)?;
    RoomRefreshToken::export_all(&output_dir_cfg)?;
    TokenBlacklistEntry::export_all(&output_dir_cfg)?;
    RoomUploadReservation::export_all(&output_dir_cfg)?;
    UploadFileDescriptor::export_all(&output_dir_cfg)?;
    UploadStatus::export_all(&output_dir_cfg)?;

    TokenType::export_all(&output_dir_cfg)?;
    RoomTokenClaims::export_all(&output_dir_cfg)?;

    IssueTokenRequest::export_all(&output_dir_cfg)?;
    IssueTokenResponse::export_all(&output_dir_cfg)?;
    ValidateTokenRequest::export_all(&output_dir_cfg)?;
    ValidateTokenResponse::export_all(&output_dir_cfg)?;
    UpdateRoomPermissionRequest::export_all(&output_dir_cfg)?;
    UpdateRoomSettingsRequest::export_all(&output_dir_cfg)?;
    RevokeTokenResponse::export_all(&output_dir_cfg)?;
    DeleteRoomResponse::export_all(&output_dir_cfg)?;
    RoomTokenView::export_all(&output_dir_cfg)?;

    RoomContentView::export_all(&output_dir_cfg)?;
    UploadContentResponse::export_all(&output_dir_cfg)?;
    UploadPreparationRequest::export_all(&output_dir_cfg)?;
    UploadPreparationResponse::export_all(&output_dir_cfg)?;
    DeleteContentRequest::export_all(&output_dir_cfg)?;
    DeleteContentResponse::export_all(&output_dir_cfg)?;
    UpdateContentRequest::export_all(&output_dir_cfg)?;
    UpdateContentResponse::export_all(&output_dir_cfg)?;
    CreateMessageRequest::export_all(&output_dir_cfg)?;
    CreateMessageResponse::export_all(&output_dir_cfg)?;

    ChunkedUploadPreparationRequest::export_all(&output_dir_cfg)?;
    ChunkedUploadPreparationResponse::export_all(&output_dir_cfg)?;
    ReservedFileInfo::export_all(&output_dir_cfg)?;
    ChunkUploadRequest::export_all(&output_dir_cfg)?;
    ChunkUploadResponse::export_all(&output_dir_cfg)?;
    UploadStatusQuery::export_all(&output_dir_cfg)?;
    ChunkStatusInfo::export_all(&output_dir_cfg)?;
    UploadStatusResponse::export_all(&output_dir_cfg)?;
    FileMergeRequest::export_all(&output_dir_cfg)?;
    FileMergeResponse::export_all(&output_dir_cfg)?;
    MergedFileInfo::export_all(&output_dir_cfg)?;

    LogoutRequest::export_all(&output_dir_cfg)?;
    CleanupResponse::export_all(&output_dir_cfg)?;
    FullRoomGcStatusView::export_all(&output_dir_cfg)?;
    RunRoomGcResponse::export_all(&output_dir_cfg)?;
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
        "CreateRefreshTokenRequest",
        "RefreshTokenRequest",
        "RefreshTokenResponse",
        "RoomRefreshToken",
        "TokenBlacklistEntry",
        "RoomUploadReservation",
        "UploadFileDescriptor",
        "UploadStatus",
        "TokenType",
        "RoomTokenClaims",
        "IssueTokenRequest",
        "IssueTokenResponse",
        "ValidateTokenRequest",
        "ValidateTokenResponse",
        "UpdateRoomPermissionRequest",
        "UpdateRoomSettingsRequest",
        "RevokeTokenResponse",
        "DeleteRoomResponse",
        "RoomTokenView",
        "RoomContentView",
        "UploadContentResponse",
        "UploadPreparationRequest",
        "UploadPreparationResponse",
        "DeleteContentRequest",
        "DeleteContentResponse",
        "UpdateContentRequest",
        "UpdateContentResponse",
        "CreateMessageRequest",
        "CreateMessageResponse",
        "ChunkedUploadPreparationRequest",
        "ChunkedUploadPreparationResponse",
        "ReservedFileInfo",
        "ChunkUploadRequest",
        "ChunkUploadResponse",
        "UploadStatusQuery",
        "ChunkStatusInfo",
        "UploadStatusResponse",
        "FileMergeRequest",
        "FileMergeResponse",
        "MergedFileInfo",
        "LogoutRequest",
        "CleanupResponse",
        "FullRoomGcStatusView",
        "RunRoomGcResponse",
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
        create_refresh_token_request: CreateRefreshTokenRequest,
        refresh_token_request: RefreshTokenRequest,
        refresh_token_response: RefreshTokenResponse,
        room_refresh_token: RoomRefreshToken,
        token_blacklist_entry: TokenBlacklistEntry,
        room_upload_reservation: RoomUploadReservation,
        upload_file_descriptor: UploadFileDescriptor,
        upload_status: UploadStatus,
        token_type: TokenType,
        room_token_claims: RoomTokenClaims,
        issue_token_request: IssueTokenRequest,
        issue_token_response: IssueTokenResponse,
        validate_token_request: ValidateTokenRequest,
        validate_token_response: ValidateTokenResponse,
        update_room_permission_request: UpdateRoomPermissionRequest,
        update_room_settings_request: UpdateRoomSettingsRequest,
        revoke_token_response: RevokeTokenResponse,
        delete_room_response: DeleteRoomResponse,
        room_token_view: RoomTokenView,
        room_content_view: RoomContentView,
        upload_content_response: UploadContentResponse,
        upload_preparation_request: UploadPreparationRequest,
        upload_preparation_response: UploadPreparationResponse,
        delete_content_request: DeleteContentRequest,
        delete_content_response: DeleteContentResponse,
        update_content_request: UpdateContentRequest,
        update_content_response: UpdateContentResponse,
        create_message_request: CreateMessageRequest,
        create_message_response: CreateMessageResponse,
        chunked_upload_preparation_request: ChunkedUploadPreparationRequest,
        chunked_upload_preparation_response: ChunkedUploadPreparationResponse,
        reserved_file_info: ReservedFileInfo,
        chunk_upload_request: ChunkUploadRequest,
        chunk_upload_response: ChunkUploadResponse,
        upload_status_query: UploadStatusQuery,
        chunk_status_info: ChunkStatusInfo,
        upload_status_response: UploadStatusResponse,
        file_merge_request: FileMergeRequest,
        file_merge_response: FileMergeResponse,
        merged_file_info: MergedFileInfo,
        logout_request: LogoutRequest,
        cleanup_response: CleanupResponse,
        full_room_gc_status_view: FullRoomGcStatusView,
        run_room_gc_response: RunRoomGcResponse,
    }

    let root = schema_for!(ApiSchema);
    serde_json::to_string_pretty(&root)
}
