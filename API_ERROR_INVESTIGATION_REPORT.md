# API 错误调查报告

## 问题现象

在获取房间消息时，前端控制台报出以下错误：

```
Console APIError
Bad Request
lib/utils/api.ts (367:17) @ request
components/layout/middle-column.tsx (65:35) @ fetchMessages
```

堆栈跟踪：

- `getMessages` 函数调用 `api.get()`
- API 层返回 400 Bad Request 错误
- 错误在第 367 行被捕获（JSON 解析错误）

## 根本原因分析

### 问题 1: 房间标识符不匹配（HIGH PRIORITY）

**症状**

- 后端和前端使用不同的房间标识符进行 API 调用

**分析**

```
前端调用: GET /api/v1/rooms/test-room-permissions_<UUID>/contents?token=...
后端期望: GET /api/v1/rooms/test-room-permissions/contents?token=...
```

当权限被修改为非分享状态时：

- 房间的 `slug` 字段被更新为 `test-room-permissions_<UUID>` 格式
- 前端在 URL 栏中显示新 slug
- 但后端的 `find_by_name()` 可能无法正确识别使用 slug 查询的房间

**代码位置**

- 前端：`web/components/layout/middle-column.tsx` (65 行) - 使用 `currentRoomId`
  调用 `getMessages()`
- 前端：`web/api/messageService.ts` (67 行) - 调用
  `API_ENDPOINTS.content.base(roomName)`
- 后端：`crates/board/src/handlers/rooms.rs` - 房间查询逻辑

### 问题 2: 房间查询逻辑不支持 Slug（MEDIUM PRIORITY）

**分析** 房间查询使用 `find_by_name()` 方法，可能不支持使用 slug 进行查询：

```rust
let room = room_repo
    .find_by_name(room_name)  // 可能只查询 'name' 字段，不查询 'slug'
    .await?
```

如果数据库中房间记录为：

```
name: 'test-room-permissions'
slug: 'test-room-permissions_40543cf5-9e78-4d6f-bff8-accccdd0a624'
```

而请求使用 slug 进行查询，将返回 404 或验证失败，最终返回 400。

### 问题 3: 错误处理链中的问题（MEDIUM PRIORITY）

**分析** 在 `lib/utils/api.ts` 第 359-372 行：

```typescript
try {
  const errorData = await response.json();
  throw new APIError(
    errorData.message || response.statusText,
    errorData.code || response.status,
    response,
  );
} catch (parseError) {
  throw new APIError(
    response.statusText || "Request failed", // 第 367 行 - 这里的 parseError 未被捕获
    response.status,
    response,
  );
}
```

如果后端返回的不是有效 JSON（或不是预期的错误格式），JSON 解析会失败，捕获的
`parseError` 被忽略，错误信息变得模糊。

## 影响范围

受影响的 API 调用：

- ✅ 获取消息 (`GET /rooms/{id}/contents`)
- ✅ 发送消息 (`POST /rooms/{id}/contents/prepare`)
- ✅ 删除消息 (`DELETE /rooms/{id}/contents`)
- ✅ 更新消息 (`PUT /rooms/{id}/contents/{id}`)
- ✅ 房间权限更新后的所有后续操作

## 解决方案

### 方案 1: 后端支持 Slug 查询（推荐）

修改后端的 `find_by_name()` 方法，使其同时支持房间名称和 slug 查询：

```rust
pub async fn find_by_name_or_slug(identifier: &str) -> Result<Option<Room>> {
    // 首先尝试按 name 查询
    if let Some(room) = sqlx::query_as::<_, Room>(
        "SELECT * FROM rooms WHERE name = ?"
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await?
    {
        return Ok(Some(room));
    }

    // 如果没找到，按 slug 查询
    sqlx::query_as::<_, Room>(
        "SELECT * FROM rooms WHERE slug = ?"
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await
}
```

### 方案 2: 前端在权限改变后更新 currentRoomId

修改前端逻辑，权限保存成功后更新 `currentRoomId` 为新的 slug：

在 `web/components/room/room-permissions.tsx`:

```typescript
onSuccess: (async (updatedRoom) => {
  const newSlug = updatedRoom.slug || updatedRoom.name;
  const oldSlug = currentRoomId;

  if (newSlug !== oldSlug) {
    // 更新本地存储的房间 ID
    setCurrentRoomId(newSlug);
    // 更新 URL
    window.location.href = `/${newSlug}`;
  }
});
```

### 方案 3: 改进错误处理和日志

在 `web/lib/utils/api.ts` 第 359-372 行改进：

```typescript
try {
  const errorData = await response.json();
  throw new APIError(
    errorData.message || response.statusText,
    errorData.code || response.status,
    response,
  );
} catch (parseError) {
  // 记录原始错误以便调试
  console.error("Response parse error:", {
    status: response.status,
    statusText: response.statusText,
    parseError,
  });

  throw new APIError(
    `HTTP ${response.status}: ${response.statusText || "Request failed"}`,
    response.status,
    response,
  );
}
```

## 建议的修复优先级

1. **高优先级**: 方案 1 - 修复后端支持 slug 查询
   - 这是最根本的解决方案
   - 处理 URL 中使用 slug 的所有情况
   - 代码改动最少

2. **中优先级**: 方案 3 - 改进错误处理
   - 提供更清晰的错误信息
   - 便于未来调试
   - 用户能获得更好的错误提示

3. **低优先级**: 方案 2 - 前端同步
   - 作为补充方案
   - 改进用户体验
   - 防止路由不一致

## 验证步骤

修复后的验证流程：

1. 创建房间
2. 禁用分享权限
3. 观察 slug 变更为 UUID 格式
4. 尝试获取消息 - **应该成功**（不再报 400 错误）
5. 尝试发送消息 - **应该成功**
6. 尝试编辑/删除消息 - **应该成功**
