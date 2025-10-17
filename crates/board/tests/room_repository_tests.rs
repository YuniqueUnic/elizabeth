//! RoomRepository 单元测试
//!
//! 测试 RoomRepository trait 的所有方法实现

use anyhow::Result;
use chrono::DateTime;
use sqlx::SqlitePool;
use std::sync::Arc;

use board::models::room::{Room, RoomStatus};
use board::repository::room_repository::{RoomRepository, SqliteRoomRepository};

/// 创建测试数据库连接池
async fn create_test_pool() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;

    // 运行迁移
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// 创建测试用的 Room 实例
fn create_test_room(name: &str) -> Room {
    let now = DateTime::from_timestamp(1609459200, 0).unwrap().naive_utc(); // 2021-01-01 00:00:00
    Room {
        id: None,
        name: name.to_string(),
        password: Some("test_password".to_string()),
        status: RoomStatus::Open,
        max_size: 100,
        current_size: 0,
        max_times_entered: 1000,
        current_times_entered: 0,
        expire_at: None,
        created_at: now,
        updated_at: now,
        permission: RoomPermission::new().with_all(), // 所有权限都允许
    }
}

#[cfg(test)]
mod room_repository_tests {
    use super::*;

    #[tokio::test]
    async fn test_room_exists() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 测试不存在的房间
        let exists = repository.exists("nonexistent_room").await?;
        assert!(!exists, "不存在的房间应该返回 false");

        // 创建房间并测试存在性
        let room = create_test_room("test_room");
        repository.create(&room).await?;

        let exists = repository.exists("test_room").await?;
        assert!(exists, "存在的房间应该返回 true");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_room() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        let room = create_test_room("new_room");
        let created_room = repository.create(&room).await?;

        // 验证创建的房间有 ID
        assert!(created_room.id.is_some(), "创建的房间应该有 ID");
        assert_eq!(created_room.name, "new_room");
        assert_eq!(created_room.password, Some("test_password".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_name() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 测试查找不存在的房间
        let found_room = repository.find_by_name("nonexistent").await?;
        assert!(found_room.is_none(), "不存在的房间应该返回 None");

        // 创建房间并查找
        let room = create_test_room("find_test");
        let created_room = repository.create(&room).await?;

        let found_room = repository.find_by_name("find_test").await?;
        assert!(found_room.is_some(), "存在的房间应该被找到");

        let found_room = found_room.unwrap();
        assert_eq!(found_room.id, created_room.id);
        assert_eq!(found_room.name, "find_test");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 创建房间
        let room = create_test_room("id_test");
        let created_room = repository.create(&room).await?;
        let room_id = created_room.id.unwrap();

        // 查找房间
        let found_room = repository.find_by_id(room_id).await?;
        assert!(found_room.is_some(), "通过 ID 应该能找到房间");

        let found_room = found_room.unwrap();
        assert_eq!(found_room.id, Some(room_id));
        assert_eq!(found_room.name, "id_test");

        // 测试查找不存在的 ID
        let not_found = repository.find_by_id(99999).await?;
        assert!(not_found.is_none(), "不存在的 ID 应该返回 None");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_room() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 创建房间
        let mut room = create_test_room("update_test");
        let created_room = repository.create(&room).await?;

        // 修改房间信息
        room.id = created_room.id;
        room.max_size = 200;
        room.permission = 0; // 移除所有权限

        let updated_room = repository.update(&room).await?;

        assert_eq!(updated_room.id, created_room.id);
        assert_eq!(updated_room.name, "update_test");
        assert_eq!(updated_room.max_size, 200);
        assert_eq!(updated_room.permission, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_room() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 创建房间
        let room = create_test_room("delete_test");
        repository.create(&room).await?;

        // 确认房间存在
        let exists_before = repository.exists("delete_test").await?;
        assert!(exists_before);

        // 删除房间
        let deleted = repository.delete("delete_test").await?;
        assert!(deleted, "删除应该成功");

        // 确认房间不存在
        let exists_after = repository.exists("delete_test").await?;
        assert!(!exists_after);

        // 测试删除不存在的房间
        let deleted_nonexistent = repository.delete("nonexistent").await?;
        assert!(!deleted_nonexistent, "删除不存在的房间应该返回 false");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_expired_rooms() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        // 创建未过期的房间
        let active_room = create_test_room("active_room");
        repository.create(&active_room).await?;

        // 创建已过期的房间
        let mut expired_room = create_test_room("expired_room");
        expired_room.expire_at = Some(DateTime::from_timestamp(1609459200, 0).unwrap().naive_utc()); // 2021-01-01
        repository.create(&expired_room).await?;

        // 获取过期房间列表
        let expired_rooms = repository.list_expired().await?;

        // 应该只包含过期房间
        assert_eq!(expired_rooms.len(), 1);
        assert_eq!(expired_rooms[0].name, "expired_room");

        Ok(())
    }

    #[tokio::test]
    async fn test_duplicate_room_creation() -> Result<()> {
        let pool = create_test_pool().await?;
        let repository = SqliteRoomRepository::new(Arc::new(pool));

        let room = create_test_room("duplicate_test");

        // 第一次创建应该成功
        repository.create(&room).await?;

        // 第二次创建同名房间应该失败
        let result = repository.create(&room).await;
        assert!(result.is_err(), "创建同名房间应该失败");

        Ok(())
    }
}
