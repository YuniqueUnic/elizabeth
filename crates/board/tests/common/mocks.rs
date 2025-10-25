use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
/// Mock 对象和辅助工具
///
/// 提供测试用的模拟实现
use std::collections::HashMap;
use std::sync::Arc;

use board::models::permission::RoomPermission;
use board::models::{Room, RoomToken};
use board::repository::{
    IRoomContentRepository, IRoomRefreshTokenRepository, IRoomRepository, IRoomTokenRepository,
    IRoomUploadReservationRepository, ITokenBlacklistRepository,
};
use board::services::RoomTokenClaims;
use board::services::auth_service::AuthService;
use board::services::token::RoomTokenService;

/// 内存中的房间仓库模拟
#[derive(Debug, Default)]
pub struct MockRoomRepository {
    rooms: Arc<std::sync::Mutex<HashMap<String, Room>>>,
}

impl MockRoomRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rooms(rooms: HashMap<String, Room>) -> Self {
        Self {
            rooms: Arc::new(std::sync::Mutex::new(rooms)),
        }
    }

    pub fn add_room(&self, room: Room) {
        let mut rooms = self.rooms.lock().unwrap();
        rooms.insert(room.name.clone(), room);
    }

    pub fn get_room(&self, name: &str) -> Option<Room> {
        let rooms = self.rooms.lock().unwrap();
        rooms.get(name).cloned()
    }
}

#[async_trait::async_trait]
impl IRoomRepository for MockRoomRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        Ok(self.get_room(name))
    }

    async fn find_by_display_name(&self, name: &str) -> Result<Option<Room>> {
        Ok(self.get_room(name))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Room>> {
        let rooms = self.rooms.lock().unwrap();
        Ok(rooms.values().find(|r| r.id == Some(id)).cloned())
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let mut rooms = self.rooms.lock().unwrap();
        let id = rooms.len() as i64 + 1;
        let mut new_room = room.clone();
        new_room.id = Some(id);
        rooms.insert(room.name.clone(), new_room.clone());
        Ok(new_room)
    }

    async fn update(&self, room: &Room) -> Result<Room> {
        let mut rooms = self.rooms.lock().unwrap();
        if let Some(existing) = rooms.get_mut(&room.name) {
            *existing = room.clone();
            Ok(room.clone())
        } else {
            Err(anyhow::anyhow!("Room not found"))
        }
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let mut rooms = self.rooms.lock().unwrap();
        Ok(rooms.remove(name).is_some())
    }

    async fn delete_expired_before(&self, _before: NaiveDateTime) -> Result<u64> {
        // 简化实现，返回 0 表示没有删除任何记录
        Ok(0)
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let rooms = self.rooms.lock().unwrap();
        Ok(rooms.contains_key(name))
    }

    async fn list_expired(&self) -> Result<Vec<Room>> {
        let rooms = self.rooms.lock().unwrap();
        let now = Utc::now().naive_utc();
        Ok(rooms
            .values()
            .filter(|r| {
                r.expire_at
                    .map(|expire_at| expire_at < now)
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }
}

/// 简单的测试令牌服务
pub struct MockTokenService {
    secret: Arc<String>,
}

impl MockTokenService {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: Arc::new(secret.to_string()),
        }
    }

    pub fn get_secret(&self) -> &Arc<String> {
        &self.secret
    }
}

/// Mock TokenService trait for testing
pub trait MockRoomTokenService {
    fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)>;
    fn decode(&self, token: &str) -> Result<RoomTokenClaims>;
}

impl MockRoomTokenService for MockTokenService {
    fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
        use chrono::Utc;
        use jsonwebtoken::{EncodingKey, Header, encode};

        let claims = RoomTokenClaims {
            sub: room.name.clone(),
            room_id: room.id.unwrap_or(1),
            room_name: room.name.clone(),
            permission: room.permission.bits(),
            max_size: room.max_size,
            exp: Utc::now().timestamp() + 3600,
            iat: Utc::now().timestamp(),
            jti: format!("mock-{}", Utc::now().timestamp()),
            refresh_jti: None,
            token_type: board::services::token::TokenType::Access,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| anyhow::anyhow!("Token encoding failed: {}", e))?;

        Ok((token, claims))
    }

    fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
        use jsonwebtoken::{DecodingKey, Validation, decode};

        let token_data = decode::<RoomTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| anyhow::anyhow!("Token decoding failed: {}", e))?;

        Ok(token_data.claims)
    }
}

/// Mock 认证服务
#[derive(Clone)]
pub struct MockAuthService {
    token_service: Arc<MockTokenService>,
}

impl MockAuthService {
    pub fn new(secret: &str) -> Self {
        Self {
            token_service: Arc::new(MockTokenService::new(secret)),
        }
    }

    pub fn token_service(&self) -> &MockTokenService {
        &self.token_service
    }
}

/// HTTP 测试辅助工具
pub mod http {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        response::IntoResponse,
    };
    use serde_json;
    use tower::util::ServiceExt;

    /// 创建测试请求
    pub fn create_request(method: Method, uri: &str, body: Option<Body>) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);

        // 如果有请求体，设置 Content-Type
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }

        let request = if let Some(b) = body {
            builder.body(b).expect("Failed to build request with body")
        } else {
            builder
                .body(Body::empty())
                .expect("Failed to build request with empty body")
        };

        request
    }

    /// 发送请求并返回响应
    pub async fn send_request(
        app: axum::Router,
        request: Request<Body>,
    ) -> Result<axum::response::Response, anyhow::Error> {
        app.oneshot(request)
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))
    }

    /// 验证响应状态
    pub fn assert_status(response: &axum::response::Response, expected: StatusCode) {
        assert_eq!(response.status(), expected);
    }

    /// 验证响应 JSON
    pub async fn assert_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> Result<T, anyhow::Error> {
        use axum::body::to_bytes;

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        serde_json::from_slice(&body).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))
    }
}
