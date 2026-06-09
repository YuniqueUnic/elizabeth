use std::path::{Path, PathBuf};

use axum::Json;
use tokio::fs;

use crate::errors::AppError;
use crate::services::RoomTokenClaims;

pub(crate) type HandlerResult<T> = Result<Json<T>, AppError>;

#[derive(Clone, Copy)]
pub(crate) enum ContentPermission {
    View,
    Edit,
    Delete,
}

pub(crate) fn ensure_permission(
    claims: &RoomTokenClaims,
    room_allows: bool,
    action: ContentPermission,
) -> Result<(), AppError> {
    if !room_allows {
        return Err(AppError::permission_denied("Permission denied by room"));
    }

    let permission = claims.as_permission();
    let token_allows = match action {
        ContentPermission::View => permission.can_view(),
        ContentPermission::Edit => permission.can_edit(),
        ContentPermission::Delete => permission.can_delete(),
    };

    if !token_allows {
        return Err(AppError::permission_denied("Permission denied by token"));
    }

    Ok(())
}

/// 确保房间存储目录存在，使用 room_id 作为目录名
pub(crate) async fn ensure_room_storage(
    base_dir: &Path,
    room_id: i64,
) -> Result<PathBuf, std::io::Error> {
    let dir = base_dir.join(room_id.to_string());
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

pub(crate) fn room_id_or_error(claims: &RoomTokenClaims) -> Result<i64, AppError> {
    if claims.room_id <= 0 {
        return Err(AppError::internal("Room id missing in claims"));
    }
    Ok(claims.room_id)
}
