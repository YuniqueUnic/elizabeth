use crate::models::room::Room;
use crate::models::room::permission::RoomPermission;
use crate::services::auth_service::AuthService;
/// 权限验证辅助模块
///
/// 提供统一的权限验证接口和常用权限检查方法
use anyhow::Result;

/// 权限验证器
pub struct PermissionValidator<'a> {
    auth_service: &'a AuthService,
    room: &'a Room,
}

impl<'a> PermissionValidator<'a> {
    /// 创建新的权限验证器
    pub fn new(auth_service: &'a AuthService, room: &'a Room) -> Self {
        Self { auth_service, room }
    }

    /// 验证令牌是否具有指定权限
    pub async fn verify_token_permission(
        &self,
        token: &str,
        required_permission: RoomPermission,
    ) -> Result<crate::services::RoomTokenClaims> {
        self.auth_service
            .verify_token_with_room_permission(token, self.room, required_permission)
            .await
    }

    /// 验证授权头是否具有指定权限
    pub async fn verify_auth_header_permission(
        &self,
        auth_header: &str,
        required_permission: RoomPermission,
    ) -> Result<crate::services::RoomTokenClaims> {
        self.auth_service
            .verify_auth_header_with_permission(auth_header, self.room, required_permission)
            .await
    }

    /// 检查令牌是否可以编辑房间
    pub async fn can_edit(&self, token: &str) -> Result<crate::services::RoomTokenClaims> {
        self.verify_token_permission(token, RoomPermission::EDITABLE)
            .await
    }

    /// 检查令牌是否可以分享房间
    pub async fn can_share(&self, token: &str) -> Result<crate::services::RoomTokenClaims> {
        self.verify_token_permission(token, RoomPermission::SHARE)
            .await
    }

    /// 检查令牌是否可以删除房间内容
    pub async fn can_delete(&self, token: &str) -> Result<crate::services::RoomTokenClaims> {
        self.verify_token_permission(token, RoomPermission::DELETE)
            .await
    }

    /// 检查令牌是否具有所有权限
    pub async fn can_do_all(&self, token: &str) -> Result<crate::services::RoomTokenClaims> {
        self.verify_token_permission(
            token,
            RoomPermission::EDITABLE | RoomPermission::SHARE | RoomPermission::DELETE,
        )
        .await
    }
}

/// 权限构建器，用于从请求参数构建权限对象
pub struct PermissionBuilder {
    permission: RoomPermission,
}

impl PermissionBuilder {
    /// 创建新的权限构建器（默认为只读权限）
    pub fn new() -> Self {
        Self {
            permission: RoomPermission::VIEW_ONLY,
        }
    }

    /// 添加编辑权限
    pub fn with_edit(mut self) -> Self {
        self.permission = self.permission.with_edit();
        self
    }

    /// 添加分享权限
    pub fn with_share(mut self) -> Self {
        self.permission = self.permission.with_share();
        self
    }

    /// 添加删除权限
    pub fn with_delete(mut self) -> Self {
        self.permission = self.permission.with_delete();
        self
    }

    /// 添加所有权限
    pub fn with_all(mut self) -> Self {
        self.permission = self.permission.with_all();
        self
    }

    /// 构建权限对象
    pub fn build(self) -> RoomPermission {
        self.permission
    }
}

impl Default for PermissionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 常用权限组合
pub mod presets {
    use super::RoomPermission;

    /// 只读权限
    pub fn readonly() -> RoomPermission {
        RoomPermission::VIEW_ONLY
    }

    /// 编辑权限（包含查看）
    pub fn editor() -> RoomPermission {
        RoomPermission::new().with_edit()
    }

    /// 完整权限（查看、编辑、分享、删除）
    pub fn full() -> RoomPermission {
        RoomPermission::new().with_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_builder() {
        let permission = PermissionBuilder::new().with_edit().with_share().build();

        assert!(permission.can_view());
        assert!(permission.can_edit());
        assert!(permission.can_share());
        assert!(!permission.can_delete());
    }

    #[test]
    fn test_permission_presets() {
        let readonly = presets::readonly();
        assert!(readonly.can_view());
        assert!(!readonly.can_edit());

        let editor = presets::editor();
        assert!(editor.can_view());
        assert!(editor.can_edit());
        assert!(!editor.can_share());

        let full = presets::full();
        assert!(full.can_view());
        assert!(full.can_edit());
        assert!(full.can_share());
        assert!(full.can_delete());
    }
}
