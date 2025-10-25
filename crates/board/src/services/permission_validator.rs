use anyhow::Result;
use async_trait::async_trait;

use crate::models::Room;
use crate::models::room::permission::RoomPermission;
use crate::services::RoomTokenClaims;

/// 权限验证器 Trait
/// 统一的权限验证接口，支持分层设计和单一职责原则
#[async_trait]
pub trait PermissionValidator: Send + Sync {
    /// 验证令牌并检查指定权限
    async fn verify_token_with_permission(
        &self,
        token: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims>;

    /// 验证授权头并检查指定权限
    async fn verify_auth_header_with_permission(
        &self,
        auth_header: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims>;

    /// 从授权头中提取令牌
    fn extract_token_from_header(&self, auth_header: &str) -> Result<String>;
}

/// 常用权限枚举
/// 提供预定义的权限组合，简化权限验证调用
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// 只读权限
    View,
    /// 编辑权限
    Edit,
    /// 上传权限
    Upload,
    /// 下载权限
    Download,
    /// 删除权限
    Delete,
    /// 分享权限
    Share,
    /// 管理权限（包含所有权限）
    Manage,
    /// 所有权限
    All,
}

impl Permission {
    /// 转换为 RoomPermission
    pub fn to_room_permission(self) -> RoomPermission {
        match self {
            Permission::View => RoomPermission::VIEW_ONLY,
            Permission::Edit => RoomPermission::EDITABLE,
            Permission::Upload => RoomPermission::EDITABLE, // 上传需要编辑权限
            Permission::Download => RoomPermission::VIEW_ONLY, // 下载只需要查看权限
            Permission::Delete => RoomPermission::DELETE,
            Permission::Share => RoomPermission::SHARE,
            Permission::Manage => RoomPermission::all(),
            Permission::All => RoomPermission::all(),
        }
    }

    /// 检查权限是否满足要求
    pub fn satisfies(self, user_permission: &RoomPermission) -> bool {
        let required = self.to_room_permission();
        user_permission.contains(required)
    }
}

/// 权限验证器工厂
/// 创建不同类型的权限验证器
pub struct PermissionValidatorFactory;

impl PermissionValidatorFactory {
    /// 创建默认的权限验证器
    pub fn create_default(
        auth_service: std::sync::Arc<crate::services::AuthService>,
    ) -> DefaultPermissionValidator {
        DefaultPermissionValidator { auth_service }
    }

    /// 创建缓存的权限验证器
    pub fn create_cached(
        auth_service: std::sync::Arc<crate::services::AuthService>,
        cache_size: usize,
    ) -> CachedPermissionValidator {
        CachedPermissionValidator {
            auth_service,
            cache: std::sync::Arc::new(tokio::sync::Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(cache_size).unwrap(),
            ))),
        }
    }
}

/// 默认权限验证器实现
/// 直接委托给 AuthService，不进行任何缓存
pub struct DefaultPermissionValidator {
    auth_service: std::sync::Arc<crate::services::AuthService>,
}

#[async_trait]
impl PermissionValidator for DefaultPermissionValidator {
    async fn verify_token_with_permission(
        &self,
        token: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims> {
        self.auth_service
            .verify_token_with_room_permission(token, room, required_permission)
            .await
    }

    async fn verify_auth_header_with_permission(
        &self,
        auth_header: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims> {
        self.auth_service
            .verify_auth_header_with_permission(auth_header, room, required_permission)
            .await
    }

    fn extract_token_from_header(&self, auth_header: &str) -> Result<String> {
        self.auth_service.extract_token_from_header(auth_header)
    }
}

/// 缓存权限验证器实现
/// 对权限验证结果进行缓存，提高性能
#[allow(clippy::type_complexity)]
pub struct CachedPermissionValidator {
    auth_service: std::sync::Arc<crate::services::AuthService>,
    cache: std::sync::Arc<
        tokio::sync::Mutex<lru::LruCache<String, (RoomTokenClaims, chrono::DateTime<chrono::Utc>)>>,
    >,
}

#[async_trait]
impl PermissionValidator for CachedPermissionValidator {
    async fn verify_token_with_permission(
        &self,
        token: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims> {
        let cache_key = format!(
            "{}:{}:{}",
            token,
            room.id.unwrap_or_default(),
            required_permission.bits()
        );

        // 检查缓存
        {
            let mut cache = self.cache.lock().await;
            if let Some((claims, cached_at)) = cache.get(&cache_key) {
                // 检查缓存是否仍然有效（5 分钟内的缓存）
                let now = chrono::Utc::now();
                let cache_age = now.signed_duration_since(*cached_at);
                if cache_age.num_minutes() < 5 && !claims.is_expired() {
                    return Ok(claims.clone());
                }
            }
        }

        // 执行实际验证
        let claims = self
            .auth_service
            .verify_token_with_room_permission(token, room, required_permission)
            .await?;

        // 更新缓存
        {
            let mut cache = self.cache.lock().await;
            cache.put(cache_key, (claims.clone(), chrono::Utc::now()));
        }

        Ok(claims)
    }

    async fn verify_auth_header_with_permission(
        &self,
        auth_header: &str,
        room: &Room,
        required_permission: RoomPermission,
    ) -> Result<RoomTokenClaims> {
        let token = self.extract_token_from_header(auth_header)?;
        self.verify_token_with_permission(&token, room, required_permission)
            .await
    }

    fn extract_token_from_header(&self, auth_header: &str) -> Result<String> {
        self.auth_service.extract_token_from_header(auth_header)
    }
}

/// 为 DefaultPermissionValidator 提供便捷方法
impl DefaultPermissionValidator {
    /// 验证令牌并检查预定义权限
    pub async fn verify_token_with_enum_permission(
        &self,
        token: &str,
        room: &Room,
        permission: Permission,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_permission(token, room, permission.to_room_permission())
            .await
    }

    /// 验证授权头并检查预定义权限
    pub async fn verify_auth_header_with_enum_permission(
        &self,
        auth_header: &str,
        room: &Room,
        permission: Permission,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_permission(auth_header, room, permission.to_room_permission())
            .await
    }

    /// 验证查看权限
    pub async fn verify_token_with_view_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::View)
            .await
    }

    /// 验证编辑权限
    pub async fn verify_token_with_edit_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Edit)
            .await
    }

    /// 验证上传权限
    pub async fn verify_token_with_upload_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Upload)
            .await
    }

    /// 验证下载权限
    pub async fn verify_token_with_download_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Download)
            .await
    }

    /// 验证删除权限
    pub async fn verify_token_with_delete_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Delete)
            .await
    }

    /// 验证管理权限
    pub async fn verify_token_with_manage_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Manage)
            .await
    }

    /// 验证授权头查看权限
    pub async fn verify_auth_header_with_view_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::View)
            .await
    }

    /// 验证授权头编辑权限
    pub async fn verify_auth_header_with_edit_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Edit)
            .await
    }

    /// 验证授权头上传权限
    pub async fn verify_auth_header_with_upload_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Upload)
            .await
    }

    /// 验证授权头下载权限
    pub async fn verify_auth_header_with_download_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Download)
            .await
    }

    /// 验证授权头删除权限
    pub async fn verify_auth_header_with_delete_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Delete)
            .await
    }

    /// 验证授权头管理权限
    pub async fn verify_auth_header_with_manage_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Manage)
            .await
    }
}

/// 为 CachedPermissionValidator 提供便捷方法
impl CachedPermissionValidator {
    /// 验证令牌并检查预定义权限
    pub async fn verify_token_with_enum_permission(
        &self,
        token: &str,
        room: &Room,
        permission: Permission,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_permission(token, room, permission.to_room_permission())
            .await
    }

    /// 验证授权头并检查预定义权限
    pub async fn verify_auth_header_with_enum_permission(
        &self,
        auth_header: &str,
        room: &Room,
        permission: Permission,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_permission(auth_header, room, permission.to_room_permission())
            .await
    }

    /// 验证查看权限
    pub async fn verify_token_with_view_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::View)
            .await
    }

    /// 验证编辑权限
    pub async fn verify_token_with_edit_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Edit)
            .await
    }

    /// 验证上传权限
    pub async fn verify_token_with_upload_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Upload)
            .await
    }

    /// 验证下载权限
    pub async fn verify_token_with_download_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Download)
            .await
    }

    /// 验证删除权限
    pub async fn verify_token_with_delete_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Delete)
            .await
    }

    /// 验证管理权限
    pub async fn verify_token_with_manage_permission(
        &self,
        token: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_token_with_enum_permission(token, room, Permission::Manage)
            .await
    }

    /// 验证授权头查看权限
    pub async fn verify_auth_header_with_view_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::View)
            .await
    }

    /// 验证授权头编辑权限
    pub async fn verify_auth_header_with_edit_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Edit)
            .await
    }

    /// 验证授权头上传权限
    pub async fn verify_auth_header_with_upload_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Upload)
            .await
    }

    /// 验证授权头下载权限
    pub async fn verify_auth_header_with_download_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Download)
            .await
    }

    /// 验证授权头删除权限
    pub async fn verify_auth_header_with_delete_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Delete)
            .await
    }

    /// 验证授权头管理权限
    pub async fn verify_auth_header_with_manage_permission(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        self.verify_auth_header_with_enum_permission(auth_header, room, Permission::Manage)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Room;
    use crate::models::room::permission::RoomPermission;
    use crate::services::AuthService;
    use std::sync::Arc;

    // 测试权限枚举转换
    #[test]
    fn test_permission_conversion() {
        assert_eq!(
            Permission::View.to_room_permission(),
            RoomPermission::VIEW_ONLY
        );
        assert_eq!(
            Permission::Edit.to_room_permission(),
            RoomPermission::EDITABLE
        );
        assert_eq!(
            Permission::Upload.to_room_permission(),
            RoomPermission::EDITABLE
        );
        assert_eq!(
            Permission::Download.to_room_permission(),
            RoomPermission::VIEW_ONLY
        );
        assert_eq!(
            Permission::Delete.to_room_permission(),
            RoomPermission::DELETE
        );
        assert_eq!(
            Permission::Share.to_room_permission(),
            RoomPermission::SHARE
        );
        assert_eq!(
            Permission::Manage.to_room_permission(),
            RoomPermission::all()
        );
        assert_eq!(Permission::All.to_room_permission(), RoomPermission::all());
    }

    // 测试权限满足检查
    #[test]
    fn test_permission_satisfies() {
        let all_permissions = RoomPermission::all();
        assert!(Permission::View.satisfies(&all_permissions));
        assert!(Permission::Edit.satisfies(&all_permissions));
        assert!(Permission::Delete.satisfies(&all_permissions));

        let view_only = RoomPermission::VIEW_ONLY;
        assert!(Permission::View.satisfies(&view_only));
        assert!(!Permission::Edit.satisfies(&view_only));
        assert!(!Permission::Delete.satisfies(&view_only));

        let editable = RoomPermission::EDITABLE;
        assert!(Permission::View.satisfies(&editable)); // EDITABLE 包含 VIEW_ONLY
        assert!(Permission::Edit.satisfies(&editable));
        assert!(Permission::Upload.satisfies(&editable)); // Upload 使用 EDITABLE 权限
        assert!(!Permission::Delete.satisfies(&editable));
    }
}
