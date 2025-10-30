# API 错误 400 Bad Request 问题总结

## 📋 问题描述

用户在禁用房间的"分享"权限后，尝试获取消息时收到 API 错误：

```
Console APIError
Bad Request (400)
lib/utils/api.ts (367:17)
components/layout/middle-column.tsx (65:35)
```

## 🔍 根本原因

### 主要原因：房间 Slug 查询问题

当禁用"分享"权限时，系统生成的私有 slug 格式为：

```
test-room-permissions_40543cf5-9e78-4d6f-bff8-accccdd0a624
```

**问题流程：**

1. 前端 URL 更新为新的 slug（包含 UUID）
2. 前端使用新 slug 调用 API：`GET /rooms/test-room-permissions_<UUID>/contents`
3. 后端 `find_by_name()` 方法只查询 `name` 字段，不查询 `slug` 字段
4. 后端无法识别这个标识符，返回 400 错误
5. 错误被 API 层捕获，并在控制台显示

## 📊 影响范围

所有后端 API 调用都会遇到此问题：

- ❌ 获取消息列表
- ❌ 发送新消息
- ❌ 编辑消息
- ❌ 删除消息
- ❌ 房间权限相关操作

## ✅ 推荐解决方案（3 个）

### 🥇 方案 1：后端支持 Slug 查询（强烈推荐）

**文件**: `crates/board/src/repository/room_repository.rs`

**改动**: 修改 `find_by_name()` 方法支持按 slug 查询

```rust
pub async fn find_by_name_or_slug(identifier: &str) -> Result<Option<Room>> {
    // 首先尝试按 name 查询
    let room = sqlx::query_as::<_, Room>(
        "SELECT * FROM rooms WHERE name = ?"
    )
    .bind(identifier)
    .fetch_optional(&self.pool)
    .await?;

    if room.is_some() {
        return Ok(room);
    }

    // 如果没找到，按 slug 查询
    sqlx::query_as::<_, Room>(
        "SELECT * FROM rooms WHERE slug = ?"
    )
    .bind(identifier)
    .fetch_optional(&self.pool)
    .await
}
```

**优点：**

- ✅ 根本解决问题
- ✅ 代码改动最少
- ✅ 统一处理所有情况

---

### 🥈 方案 2：改进错误处理和日志

**文件**: `web/lib/utils/api.ts` (第 359-372 行)

**改动**: 记录原始错误信息

```typescript
} catch (parseError) {
  // 记录原始错误以便调试
  console.error("API Error Response:", {
    status: response.status,
    statusText: response.statusText,
    body: await response.clone().text(),
    error: parseError,
  });

  throw new APIError(
    `HTTP ${response.status}: ${response.statusText || "请求失败"}`,
    response.status,
    response,
  );
}
```

**优点：**

- ✅ 更清晰的错误信息
- ✅ 便于调试
- ✅ 改进用户体验

---

### 🥉 方案 3：前端同步 Room ID

**文件**: `web/components/room/room-permissions.tsx`

**改动**: 权限保存成功后更新房间 ID

```typescript
onSuccess: (async (updatedRoom) => {
  const newSlug = updatedRoom.slug || updatedRoom.name;

  if (newSlug !== currentRoomId) {
    // 更新本地状态
    setCurrentRoomId(newSlug);

    // 刷新页面以确保同步
    window.location.href = `/${newSlug}`;
  }
});
```

**优点：**

- ✅ 防止路由不一致
- ✅ 改进用户体验

---

## 🚀 实施步骤

### 步骤 1：修复后端（必须）

修改后端仓库的查询方法以支持 slug 查询。

### 步骤 2：改进错误处理（推荐）

增强错误日志记录以便未来调试。

### 步骤 3：测试验证

```bash
1. 创建新房间
2. 禁用"分享"权限 → 观察 slug 变更
3. 尝试发送消息 → 应该成功（不报 400 错误）
4. 尝试编辑/删除消息 → 应该成功
```

## 📝 总结

| 方面         | 内容                     |
| ------------ | ------------------------ |
| **问题类型** | 房间标识符匹配问题       |
| **严重程度** | 🔴 高 - 影响所有消息操作 |
| **影响用户** | 禁用分享权限后的所有用户 |
| **修复难度** | 🟢 低 - 改动 <10 行代码  |
| **所需时间** | ~30 分钟                 |

---

## 💡 关键要点

- 后端需要支持按 name 或 slug 查询房间
- 前端应该保持 URL 和内部状态同步
- 需要改进错误处理和日志记录
