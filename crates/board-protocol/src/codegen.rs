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
    Room::export_all_to(output_dir)?;
    RoomStatus::export_all_to(output_dir)?;
    RoomContent::export_all_to(output_dir)?;
    ContentType::export_all_to(output_dir)?;
    ChunkStatus::export_all_to(output_dir)?;
    RoomChunkUpload::export_all_to(output_dir)?;
    CreateRefreshTokenRequest::export_all_to(output_dir)?;
    RefreshTokenRequest::export_all_to(output_dir)?;
    RefreshTokenResponse::export_all_to(output_dir)?;
    RoomRefreshToken::export_all_to(output_dir)?;
    TokenBlacklistEntry::export_all_to(output_dir)?;
    RoomUploadReservation::export_all_to(output_dir)?;
    UploadFileDescriptor::export_all_to(output_dir)?;
    UploadStatus::export_all_to(output_dir)?;

    TokenType::export_all_to(output_dir)?;
    RoomTokenClaims::export_all_to(output_dir)?;

    IssueTokenRequest::export_all_to(output_dir)?;
    IssueTokenResponse::export_all_to(output_dir)?;
    ValidateTokenRequest::export_all_to(output_dir)?;
    ValidateTokenResponse::export_all_to(output_dir)?;
    UpdateRoomPermissionRequest::export_all_to(output_dir)?;
    UpdateRoomSettingsRequest::export_all_to(output_dir)?;
    RevokeTokenResponse::export_all_to(output_dir)?;
    DeleteRoomResponse::export_all_to(output_dir)?;
    RoomTokenView::export_all_to(output_dir)?;

    RoomContentView::export_all_to(output_dir)?;
    UploadContentResponse::export_all_to(output_dir)?;
    UploadPreparationRequest::export_all_to(output_dir)?;
    UploadPreparationResponse::export_all_to(output_dir)?;
    DeleteContentRequest::export_all_to(output_dir)?;
    DeleteContentResponse::export_all_to(output_dir)?;
    UpdateContentRequest::export_all_to(output_dir)?;
    UpdateContentResponse::export_all_to(output_dir)?;
    CreateMessageRequest::export_all_to(output_dir)?;
    CreateMessageResponse::export_all_to(output_dir)?;

    ChunkedUploadPreparationRequest::export_all_to(output_dir)?;
    ChunkedUploadPreparationResponse::export_all_to(output_dir)?;
    ReservedFileInfo::export_all_to(output_dir)?;
    ChunkUploadRequest::export_all_to(output_dir)?;
    ChunkUploadResponse::export_all_to(output_dir)?;
    UploadStatusQuery::export_all_to(output_dir)?;
    ChunkStatusInfo::export_all_to(output_dir)?;
    UploadStatusResponse::export_all_to(output_dir)?;
    FileMergeRequest::export_all_to(output_dir)?;
    FileMergeResponse::export_all_to(output_dir)?;
    MergedFileInfo::export_all_to(output_dir)?;

    LogoutRequest::export_all_to(output_dir)?;
    CleanupResponse::export_all_to(output_dir)?;
    FullRoomGcStatusView::export_all_to(output_dir)?;
    RunRoomGcResponse::export_all_to(output_dir)?;
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
