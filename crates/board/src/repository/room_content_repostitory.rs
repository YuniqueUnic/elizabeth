use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{db::DbPool, models::content::RoomContent};

#[async_trait]
pub trait IRoomContentRepository: Send + Sync {
    async fn exists(&self, room_name: &str) -> Result<bool>;
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn find_by_room_name(&self, room_name: &str) -> Result<Option<RoomContent>>;
    async fn find_by_room_id(&self, room_id: i64) -> Result<Option<RoomContent>>;
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn delete(&self, room_name: &str) -> Result<bool>;
}

pub struct SqliteRoomContentRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomContentRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IRoomContentRepository for SqliteRoomContentRepository {
    async fn exists(&self, room_name: &str) -> Result<bool> {
        Ok(false)
    }
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
        Ok(RoomContent::dummy())
    }
    async fn find_by_room_name(&self, room_name: &str) -> Result<Option<RoomContent>> {
        Ok(None)
    }
    async fn find_by_room_id(&self, room_id: i64) -> Result<Option<RoomContent>> {
        Ok(None)
    }
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent> {
        Ok(RoomContent::dummy())
    }
    async fn delete(&self, room_name: &str) -> Result<bool> {
        Ok(false)
    }
}
