use chrono::NaiveDateTime;

use crate::models::RoomStatus;
use crate::models::room::upload_file_policy::UploadFileTypePolicy;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct FullRoomGcStatusView {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub empty_since: Option<NaiveDateTime>,
    pub cleanup_after: Option<NaiveDateTime>,
    pub max_token_expires_at: Option<NaiveDateTime>,
    pub active_connections: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunRoomGcResponse {
    pub cleaned: u32,
}

/// 平台 dashboard 统计概览（issue #196）
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminStatsResponse {
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub rooms_total: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub rooms_open: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub rooms_protected: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub contents_total: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub contents_files: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub contents_messages: i64,
    /// 逻辑占用：内容记录字节数之和（含共享引用的重复计量）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub storage_logical_bytes: i64,
    /// 物理占用：内容寻址 blob 字节数之和（去重后）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub storage_physical_bytes: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub blob_count: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub active_connections: u32,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub active_rooms: u32,
}

/// 管理视图的房间条目
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminRoomView {
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub status: RoomStatus,
    pub password_protected: bool,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub current_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub max_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub current_times_entered: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub max_times_entered: i64,
    /// 新成员入场角色（房间角色矩阵中的系统角色 key）
    pub default_role_key: String,
    /// 上传文件类型策略（any/allow/deny + 扩展名列表）
    pub upload_file_type: UploadFileTypePolicy,
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub content_count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminRoomListResponse {
    pub rooms: Vec<AdminRoomView>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub total: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub limit: u32,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminRoomDetailResponse {
    #[serde(flatten)]
    pub room: AdminRoomView,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub blob_count: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub token_count: i64,
}

/// 存储状态（不含任何凭据）
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminStorageResponse {
    /// "fs" | "s3"
    pub backend: String,
    /// "proxy" | "presigned"
    pub transfer_mode: String,
    /// backend = fs 时的存储根目录
    pub root: Option<String>,
    /// backend = s3 时的桶名（不回显 endpoint 与凭据）
    pub bucket: Option<String>,
    pub presign_base_url: Option<String>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub physical_bytes: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub logical_bytes: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub blob_count: i64,
    /// 逻辑与物理占用之差（去重节省）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub dedup_saved_bytes: i64,
}

/// 系统配置视图：静态项为配置文件值（需重启生效），运行时可写项在白名单字段中
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminConfigResponse {
    pub server_host: String,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub server_port: u16,
    /// "sqlite" | "postgresql"（不回显连接串）
    pub database_backend: String,
    pub storage_backend: String,
    pub transfer_mode: String,
    pub storage_root: String,
    pub presign_base_url: Option<String>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_default_max_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_default_max_times_entered: i64,
    pub room_default_role_key: String,
    /// 房间有效期的允许时长（秒），升序
    #[cfg_attr(feature = "typescript-export", ts(type = "number[]"))]
    pub room_expiry_allowed_ages_seconds: Vec<i64>,
    /// 新建房间的默认有效期（秒）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_expiry_default_age_seconds: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub jwt_ttl_seconds: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub jwt_refresh_ttl_seconds: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub upload_reservation_ttl_seconds: i64,
    /// 管理面是否可用：已完成管理员账号 bootstrap（存在账号）即为 true
    pub admin_api_enabled: bool,
    /// 存储去重作用域："per-room" | "global"
    pub dedup_scope: String,
    // ---- 运行时可写白名单（进程内覆盖，重启回退到配置文件） ----
    pub runtime_disallow_search_indexing: bool,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub runtime_room_default_max_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub runtime_room_default_max_times_entered: i64,
    /// 0 = 未覆盖
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub runtime_session_ttl_seconds: i64,
    /// 0 = 未覆盖
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub runtime_upload_reservation_ttl_seconds: i64,
    /// None = 未覆盖
    pub runtime_room_default_role_key: Option<String>,
    /// None = 未覆盖（覆盖时整组替换，不会出现允许列表与默认时长不一致的中间态）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub runtime_room_expiry: Option<RoomExpiryOverride>,
}

/// 管理员登录请求
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// 管理员登录结果：会话令牌（JWT）只在登录响应中出现
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminLoginResponse {
    pub username: String,
    pub token: String,
    pub expires_at: NaiveDateTime,
}

/// 当前管理员会话
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminSessionView {
    pub username: String,
}

/// 管理员修改自己的密码；改密后该账号所有既有会话立即失效
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminPasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

/// 创建管理 API key 请求；有效期缺省 = 永不过期（可随时吊销）
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminApiKeyCreateRequest {
    pub name: String,
    #[cfg_attr(feature = "typescript-export", ts(optional, type = "number"))]
    pub expires_in_secs: Option<i64>,
}

/// 管理 API key 视图（不含明文 key）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminApiKeyView {
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub id: i64,
    pub name: String,
    /// 明文 key 的前缀，用于在密钥管理器中辨认条目
    pub prefix: String,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
    pub expires_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
}

/// 创建管理 API key 结果：明文 key 仅此一次返回
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminApiKeyCreateResponse {
    #[serde(flatten)]
    pub key: AdminApiKeyView,
    pub secret: String,
}

/// 房间有效期策略：允许时长列表 + 新建房间默认时长。
/// 二者互相约束（默认必须属于允许列表），因此始终整组读写。
/// `allowed_ages_seconds` 为空数组 = 清除运行时覆盖，回退配置文件值。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RoomExpiryOverride {
    /// 允许的房间有效期（秒）：非空、全为正、严格升序
    #[cfg_attr(feature = "typescript-export", ts(type = "number[]"))]
    pub allowed_ages_seconds: Vec<i64>,
    /// 新建房间的默认有效期（秒），必须属于 allowed_ages_seconds
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub default_age_seconds: i64,
}

/// 运行时可写配置更新（管理白名单）；字段缺省 = 保持不变。
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct UpdateRuntimeConfigRequest {
    pub disallow_search_indexing: Option<bool>,
    /// 0 = 清除覆盖，回退配置文件默认值
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_default_max_size: Option<i64>,
    /// 0 = 清除覆盖，回退配置文件默认值
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_default_max_times_entered: Option<i64>,
    /// 0 = 清除覆盖，回退配置文件默认值（访问令牌有效期，秒）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub session_ttl_seconds: Option<i64>,
    /// 0 = 清除覆盖，回退配置文件默认值（上传预留有效期，秒）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub upload_reservation_ttl_seconds: Option<i64>,
    /// 新房间默认加入角色；空串 = 清除覆盖（须为系统角色 admin/editor/reader）
    pub room_default_role_key: Option<String>,
    /// 房间有效期策略；缺省 = 保持不变，空允许列表 = 清除覆盖
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub room_expiry: Option<RoomExpiryOverride>,
}

/// 平台管理铸造房间身份码：code 缺省时由服务端生成，明文仅此一次返回
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AdminMintIdentityCodeRequest {
    /// 指定身份码；缺省自动生成（6-128 位 ASCII，规则与房间级创建一致）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub code: Option<String>,
    pub role: String,
    #[cfg_attr(feature = "typescript-export", ts(optional, type = "number"))]
    pub expires_in_secs: Option<i64>,
}
