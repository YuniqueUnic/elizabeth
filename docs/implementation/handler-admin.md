# 房间管理处理器 (Admin Handler)

## 1. 简介

房间管理处理器是 Elizabeth
系统的核心管理组件，负责处理房间的创建、查询、更新、删除以及权限管理功能。该处理器实现了房间生命周期管理，包括房间状态控制、权限配置、Slug
生成和管理等。处理器支持房间的自动创建机制，当访问不存在的房间时会自动创建新房间。主要交互方包括权限验证模块、令牌服务、存储系统和数据库层。

## 2. 数据模型

### 房间模型 (Room)

- id: Option<i64> — 主键，房间唯一标识
- name: String — 房间显示名称
- slug: String — 房间唯一标识符（用于 URL 和访问）
- password: Option<String> — 房间密码（可选）
- status: RoomStatus — 房间状态（Open=0, Lock=1, Close=2）
- max_size: i64 — 最大容量限制（字节，默认 10MB）
- current_size: i64 — 当前已用容量
- max_times_entered: i64 — 最大进入次数（默认 100）
- current_times_entered: i64 — 当前进入次数
- expire_at: Option<NaiveDateTime> — 过期时间（可选）
- created_at: NaiveDateTime — 创建时间
- updated_at: NaiveDateTime — 更新时间
- permission: RoomPermission — 房间权限配置

### 房间状态枚举 (RoomStatus)

```rust
#[repr(i64)]
pub enum RoomStatus {
    Open = 0,   // 开放状态，可以进入和操作
    Lock = 1,   // 锁定状态，已进入用户可继续操作，新用户无法进入
    Close = 2,  // 关闭状态，所有人无法进入和操作
}
```

### 权限更新请求 (UpdateRoomPermissionRequest)

- edit: bool — 是否允许编辑权限
- share: bool — 是否允许分享权限
- delete: bool — 是否允许删除权限

### 令牌视图模型 (RoomTokenView)

- jti: String — 令牌唯一标识
- expires_at: NaiveDateTime — 过期时间
- revoked_at: Option<NaiveDateTime> — 撤销时间
- created_at: NaiveDateTime — 创建时间

> 数据库表：`rooms`（迁移文件：`crates/board/migrations/001_create_rooms_table.sql`）

## 3. 不变式 & 验证逻辑

### 业务规则

- 房间名称不能为空，且在系统中必须唯一
- 房间 slug 必须全局唯一，当房间可分享时 slug 与 name 相同
- 房间创建者自动获得所有权限（VIEW_ONLY + EDITABLE + SHARE + DELETE）
- 只有具有删除权限的用户才能更新房间权限
- 房间状态为 Close 时，所有操作都被禁止
- 房间过期后无法进入，但已进入的用户可以继续操作直到令牌过期
- Slug 生成规则：可分享房间使用 name，私有房间使用 `{name}_{uuid}`

### 验证逻辑

- 房间名称长度和格式验证
- 密码验证（如果设置了密码）
- 权限级别验证（只有高级权限可以修改低级权限）
- Slug 冲突检测和自动重命名
- 房间容量和访问次数限制检查

## 4. 持久化 & 索引

### 数据库表结构

```sql
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0, -- 0: open, 1: lock, 2: close
    max_size INTEGER NOT NULL DEFAULT 10485760, -- 10MB
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1 -- bitflags for permissions
);
```

### 索引设计

- 唯一索引：`slug` 字段确保房间标识符唯一性
- 普通索引：`name` 字段优化按名称查询
- 状态索引：`status` 字段优化状态过滤查询
- 过期时间索引：`expire_at` 字段优化过期房间清理

### 触发器

- 自动更新 `updated_at` 字段的触发器
- 外键约束确保数据完整性

## 5. API/Handlers

### 创建房间

- **POST** `/api/v1/rooms/{name}`
- 请求参数：房间名称、可选密码
- 响应：创建的房间信息
- 错误码：400（房间已存在）、500（服务器错误）

### 查找房间

- **GET** `/api/v1/rooms/{name}`
- 请求参数：房间名称
- 响应：房间信息（如果房间不存在且可创建，会自动创建）
- 错误码：403（房间无法进入）、500（服务器错误）

### 删除房间

- **DELETE** `/api/v1/rooms/{name}`
- 请求参数：房间名称
- 响应：删除结果
- 错误码：404（房间不存在）、410（房间已过期）、500（服务器错误）
- **安全警告**：当前实现缺乏权限验证，任何知道房间名称的用户都可以删除房间。这是一个已知的安全问题，需要在后续版本中修复。
- 过期检查：删除前会检查房间是否过期，过期房间无法删除

### 更新房间权限

- **POST** `/api/v1/rooms/{name}/permissions`
- 请求参数：房间名称、管理员 token、权限配置
- 响应：更新后的房间信息
- 错误码：400（参数错误）、401（令牌无效）、403（权限不足）

### 令牌管理相关端点

虽然令牌管理主要由令牌处理器负责，但房间管理处理器也涉及以下令牌相关端点：

- **POST** `/api/v1/rooms/{name}/tokens` - 签发房间访问令牌
- **GET** `/api/v1/rooms/{name}/tokens` - 获取房间令牌列表
- **POST** `/api/v1/rooms/{name}/tokens/validate` - 验证令牌
- **DELETE** `/api/v1/rooms/{name}/tokens/{jti}` - 撤销指定令牌

### 请求示例

```json
// 创建房间请求
POST /api/v1/rooms/myroom?password=secret123

// 权限更新请求
POST /api/v1/rooms/myroom/permissions?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
{
  "edit": true,
  "share": false,
  "delete": false
}

// 房间信息响应
{
  "id": 123,
  "name": "myroom",
  "slug": "myroom",
  "password": "secret123",
  "status": 0,
  "max_size": 10485760,
  "current_size": 512000,
  "max_times_entered": 100,
  "current_times_entered": 5,
  "expire_at": null,
  "created_at": "2023-12-01T10:00:00",
  "updated_at": "2023-12-01T10:00:00",
  "permission": 15
}
```

## 6. JWT 与权限

### 权限验证

- 使用 `verify_room_token` 函数验证 JWT 令牌
- 检查令牌中的 `permission` 字段是否包含删除权限 (`can_delete()`)
- 验证令牌的 `room_id` 与目标房间匹配
- 确保令牌未被撤销且未过期

### 权限级别

- **VIEW_ONLY (1)**: 只能查看内容
- **EDITABLE (2)**: 包含 VIEW，可以上传和编辑
- **SHARE (4)**: 包含以上，可以分享房间
- **DELETE (8)**: 包含以上，可以删除内容和修改权限

### 权限检查流程

```rust
let token_perm = verified.claims.as_permission();
if !token_perm.can_delete() {
    return Err(HttpResponse::Forbidden().message("Permission denied by token"));
}
```

## 7. 关键代码片段

### 创建房间 (crates/board/src/handlers/rooms.rs:105)

```rust
pub async fn create(
    Path(name): Path<String>,
    Query(params): Query<CreateRoomParams>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Room> {
    // 验证房间名称
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

    // 检查房间是否已存在
    if repository.exists(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        return Err(HttpResponse::BadRequest().message("Room already exists"));
    }

    // 创建新房间
    let room = Room::new(name.clone(), params.password);
    let created_room = repository.create(&room).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
    })?;

    Ok(Json(created_room))
}
```

### 查找房间（支持自动创建）(crates/board/src/handlers/rooms.rs:144)

```rust
pub async fn find(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Room> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

    match repository.find_by_name(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        Some(room) => {
            // 房间存在，检查是否可以进入
            if room.can_enter() {
                Ok(Json(room))
            } else {
                Err(HttpResponse::Forbidden().message("Room cannot be entered"))
            }
        }
        None => {
            // 房间不存在，检查是否有同名但不同 slug 的房间
            if repository.find_by_display_name(&name).await?.is_some() {
                return Err(HttpResponse::Forbidden().message("Room cannot be accessed"));
            }

            // 自动创建新房间
            let new_room = Room::new(name, None);
            let created_room = repository.create(&new_room).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
            })?;
            Ok(Json(created_room))
        }
    }
}
```

### 更新权限 (crates/board/src/handlers/rooms.rs:349)

```rust
pub async fn update_permissions(
    Path(name): Path<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomPermissionRequest>,
) -> HandlerResult<Room> {
    // 验证权限
    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    let token_perm = verified.claims.as_permission();
    if !token_perm.can_delete() {
        return Err(HttpResponse::Forbidden().message("Permission denied by token"));
    }

    // 构建新权限
    let mut new_permission = RoomPermission::VIEW_ONLY;
    if payload.edit {
        new_permission = new_permission.with_edit();
    }
    if payload.share {
        new_permission = new_permission.with_share();
    }
    if payload.delete {
        new_permission = new_permission.with_delete();
    }

    let repo = SqliteRoomRepository::new(app_state.db_pool.clone());
    let mut room = verified.room;
    let was_shareable = room.permission.can_share();
    room.permission = new_permission;

    // 处理 slug 生成逻辑
    if payload.share {
        let desired_slug = room.name.clone();
        if desired_slug != room.slug {
            let exists = repo.exists(&desired_slug).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Database error: {e}"))
            })?;
            if exists {
                return Err(HttpResponse::BadRequest().message("Slug already in use"));
            }
            room.slug = desired_slug;
        }
    } else if was_shareable || room.slug == room.name {
        // 生成私有 slug
        loop {
            let candidate = format!("{}_{}", room.name, Uuid::new_v4());
            let exists = repo.exists(&candidate).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Database error: {e}"))
            })?;
            if !exists {
                room.slug = candidate;
                break;
            }
        }
    }

    let updated_room = repo.update(&room).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to update room: {e}"))
    })?;

    Ok(Json(updated_room))
}
```

## 8. 测试要点

### 单元测试建议

- 测试房间创建逻辑（重复名称、无效参数）
- 测试权限更新逻辑（权限级别验证）
- 测试 Slug 生成逻辑（冲突处理、UUID 生成）
- 测试房间状态转换逻辑
- 测试自动创建房间的边界条件

### 集成测试建议

- 完整的房间生命周期：创建 → 权限更新 → 删除
- 并发房间创建测试
- 权限继承和验证测试
- 房间过期和状态管理测试
- Slug 冲突解决测试

### 边界条件测试

- 房间名称包含特殊字符的情况
- 权限位标志溢出的情况
- 房间状态快速切换的情况
- 大量房间创建的性能测试

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

- **房间批量操作**：当前缺乏批量管理房间的功能，建议添加批量删除、更新接口
- **房间模板功能**：缺乏预设房间模板，建议支持常用配置的快速创建

### P1 优先级

- **房间统计功能**：缺乏详细的使用统计，建议添加访问量、存储使用量等指标
- **房间导入导出**：缺乏房间配置的备份和恢复机制

### P2 优先级

- **房间分类管理**：缺乏房间分类和标签功能
- **房间继承权限**：缺乏权限继承机制，无法快速设置相似的权限配置

## 10. 关联文档 / 代码位置

### 源码路径

- 处理器实现：`crates/board/src/handlers/rooms.rs:105-484`
- 路由定义：`crates/board/src/route/room.rs:10-26`
- 数据模型：`crates/board/src/models/room/mod.rs`
- 权限模型：`crates/board/src/models/room/permission.rs`

### 数据库相关

- 迁移文件：`crates/board/migrations/001_create_rooms_table.sql`
- 索引优化：`crates/board/migrations/004_add_indexes.sql`

### 测试文件

- 集成测试：`crates/board/tests/api_integration_tests.rs`
- 仓库测试：`crates/board/tests/room_repository_tests.rs`

### 相关文档

- [权限模型文档](model-permissions.md)
- [令牌处理器文档](handler-token.md)
- [上传处理器文档](handler-upload.md)
- [下载处理器文档](handler-download.md)
