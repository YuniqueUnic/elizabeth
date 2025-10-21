# Permissions

## 1. 简介

Permissions 模型是 Elizabeth
系统的权限控制核心，采用位标志（bitflags）设计模式实现高效的权限管理。系统使用 8
位无符号整数来表示四种基本权限：预览（VIEW_ONLY）、编辑（EDITABLE）、分享（SHARE）和删除（DELETE）。权限信息存储在房间模型中，并通过
JWT 传递给客户端，实现无状态的权限验证。主要交互方包括房间模型（`Room`）、JWT
服务（`RoomTokenService`）和各个内容处理器。

## 2. 数据模型（字段 & 类型 & 解释）

**RoomPermission
位标志结构**（`crates/board/src/models/room/permission.rs:28`）：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomPermission: u8 {
    const VIEW_ONLY = 1;    // 预览权限：可以查看房间内容
    const EDITABLE = 1 << 1; // 编辑权限：可以上传和修改内容
    const SHARE = 1 << 2;   // 分享权限：房间可以被公开分享
    const DELETE = 1 << 3;  // 删除权限：可以删除房间内容
}
```

**权限位值映射**：

- `VIEW_ONLY = 1` (二进制：`0001`)：基础预览权限
- `EDITABLE = 2` (二进制：`0010`)：内容编辑权限
- `SHARE = 4` (二进制：`0100`)：房间分享权限
- `DELETE = 8` (二进制：`1000`)：内容删除权限

**权限组合示例**：

- `VIEW_ONLY | EDITABLE = 3` (二进制：`0011`)：预览 + 编辑
- `VIEW_ONLY | EDITABLE | SHARE | DELETE = 15` (二进制：`1111`)：全部权限

## 3. 不变式 & 验证逻辑（业务规则）

- **最小权限原则**：所有房间默认至少具有 `VIEW_ONLY` 权限
- **权限继承**：JWT 中的权限在签发时从房间的 `permission` 字段复制，不会动态变化
- **权限组合**：权限可以任意组合，但某些操作可能需要多个权限同时满足
- **权限验证顺序**：先验证房间级别权限，再验证 JWT 中的权限
- **权限持久化**：权限变更只影响新签发的 JWT，现有 JWT 权限不变

## 4. 持久化 & 索引（实现细节）

**数据库存储**：

```sql
-- rooms 表中的 permission 字段
CREATE TABLE IF NOT EXISTS rooms (
    -- ... 其他字段 ...
    permission INTEGER NOT NULL DEFAULT 1 -- 默认预览权限
);
```

**存储格式**：

- 数据库中存储为 `INTEGER` 类型
- 序列化时使用透明 `#[serde(transparent)]` 包装，直接存储数值
- API 文档中显示为整数类型，包含详细的位掩码说明

**数据库编码/解码**（`crates/board/src/models/room/permission.rs:99`）：

```rust
impl Type<Sqlite> for RoomPermission {
    fn type_info() -> SqliteTypeInfo {
        <u8 as Type<Sqlite>>::type_info()
    }
}

impl Encode<'_, Sqlite> for RoomPermission {
    fn encode(self, buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'_>) -> Result<IsNull, BoxDynError> {
        <u8 as Encode<Sqlite>>::encode(self.bits(), buf)
    }
}

impl Decode<'_, Sqlite> for RoomPermission {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
        let raw = <u8 as Decode<Sqlite>>::decode(value)?;
        RoomPermission::from_bits(raw)
            .ok_or_else(|| format!("invalid RoomPermission bits: {}", raw).into())
    }
}
```

## 5. API/Handlers（对外行为）

**权限相关端点**：

- `POST /api/v1/rooms/{name}/permissions` - 更新房间权限
  - 输入：`UpdateRoomPermissionRequest { edit: bool, share: bool, delete: bool }`
  - 输出：更新后的 Room 对象
  - 要求：需要具有删除权限的 JWT

**权限验证流程**：

1. **房间级别检查**：验证房间本身是否允许该操作
2. **JWT 权限检查**：验证令牌中是否包含所需权限
3. **操作执行**：两者都满足时才允许执行操作

**请求/响应示例**：

```json
// 更新权限请求
POST /api/v1/rooms/myroom/permissions?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
{
  "edit": true,
  "share": false,
  "delete": false
}

// 响应
{
  "id": 1,
  "name": "myroom",
  "permission": 3  // VIEW_ONLY | EDITABLE
}
```

## 6. JWT 与权限（如何生成/校验）

**JWT 中的权限编码**：

```rust
// JWT 签发时提取权限位
let claims = RoomTokenClaims {
    // ... 其他字段 ...
    permission: room.permission.bits(),  // 存储为 u8 数值
    // ... 其他字段 ...
};
```

**权限解码和验证**：

```rust
// 从 JWT 恢复权限对象
impl RoomTokenClaims {
    pub fn as_permission(&self) -> RoomPermission {
        RoomPermission::from_bits(self.permission).unwrap_or_default()
    }
}

// 权限检查示例
let permission = claims.as_permission();
if permission.can_edit() {
    // 允许编辑操作
}
```

**权限验证函数**（`crates/board/src/models/room/permission.rs:68`）：

```rust
impl RoomPermission {
    pub fn can_view(&self) -> bool {
        self.contains(RoomPermission::VIEW_ONLY)
    }
    pub fn can_edit(&self) -> bool {
        self.contains(RoomPermission::EDITABLE)
    }
    pub fn can_share(&self) -> bool {
        self.contains(RoomPermission::SHARE)
    }
    pub fn can_delete(&self) -> bool {
        self.contains(RoomPermission::DELETE)
    }
    pub fn can_do_all(&self) -> bool {
        self.can_view() && self.can_edit() && self.can_share() && self.can_delete()
    }
}
```

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）

**权限构建器模式**（`crates/board/src/models/room/permission.rs:44`）：

```rust
impl RoomPermission {
    pub fn new() -> Self {
        Self::default()  // 默认 VIEW_ONLY
    }
    pub fn with_edit(mut self) -> Self {
        self |= RoomPermission::EDITABLE;
        self
    }
    pub fn with_share(mut self) -> Self {
        self |= RoomPermission::SHARE;
        self
    }
    pub fn with_delete(mut self) -> Self {
        self |= RoomPermission::DELETE;
        self
    }
    pub fn with_all(mut self) -> Self {
        self |= RoomPermission::EDITABLE;
        self |= RoomPermission::SHARE;
        self |= RoomPermission::DELETE;
        self
    }
}
```

**权限验证逻辑**（`crates/board/src/handlers/content.rs:705`）：

```rust
fn ensure_permission(
    claims: &RoomTokenClaims,
    room_allows: bool,
    action: ContentPermission,
) -> Result<(), HttpResponse> {
    if !room_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by room"));
    }
    let permission = claims.as_permission();
    let token_allows = match action {
        ContentPermission::View => permission.can_view(),
        ContentPermission::Edit => permission.can_edit(),
        ContentPermission::Delete => permission.can_delete(),
    };
    if !token_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by token"));
    }
    Ok(())
}
```

**API Schema 定义**（`crates/board/src/models/room/permission.rs:86`）：

```rust
impl PartialSchema for RoomPermission {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        ObjectBuilder::new()
            .schema_type(SchemaType::from(SchemaPrimitive::Integer))
            .description(Some(
                "房间权限位掩码，使用 bitflags 表示：1=VIEW_ONLY, 2=EDITABLE, 4=SHARE, 8=DELETE。",
            ))
            .into()
    }
}
```

## 8. 测试要点（单元/集成测试建议）

- **基础功能测试**：
  - 权限位的设置、清除、检查操作
  - 权限组合的正确性验证
  - 数据库存储和读取的一致性

- **边界条件测试**：
  - 无效权限位的处理
  - 权限溢出的防护
  - 空权限的情况（虽然默认不会出现）

- **安全测试**：
  - 权限提升攻击的防护
  - JWT 篡改后的权限验证
  - 权限绕过尝试的检测

- **集成测试**：
  - 权限变更对现有 JWT 的影响
  - 不同权限组合下的操作验证
  - 权限验证的完整流程测试

## 9. 已知问题 / TODO / 改进建议

**P0 优先级**：

- **权限粒度不足**：当前权限模型较为粗糙，无法实现细粒度的文件级别权限控制
- **权限继承机制**：缺少权限继承和覆盖机制，无法支持复杂的权限层级

**P1 优先级**：

- **权限审计日志**：缺少权限变更的详细审计记录
- **权限模板支持**：不支持预定义的权限组合模板

**P2 优先级**：

- **动态权限系统**：支持基于条件的动态权限验证
- **权限组管理**：支持权限组的创建和管理
- **时间限制权限**：支持有时效性的权限设置

## 10. 关联文档 / 代码位置

**源码路径**：

- 权限模型：`crates/board/src/models/room/permission.rs`
- 权限验证：`crates/board/src/handlers/content.rs:705`
- 权限更新：`crates/board/src/handlers/rooms.rs:349`
- JWT 权限处理：`crates/board/src/services/token.rs:30`

**测试文件路径**：

- 单元测试：`crates/board/src/models/room/permission.rs:129` 中的 `#[cfg(test)]`
  块
- 集成测试：`crates/board/tests/api_integration_tests.rs`

**关联文档**：

- [model-room.md](./model-room.md) - 房间模型详细说明
- [model-session-jwt.md](./model-session-jwt.md) - JWT 令牌机制
- [model-file.md](./model-file.md) - 文件内容管理

**权限使用示例**：

```rust
// 创建具有编辑和删除权限的房间
let permission = RoomPermission::new()
    .with_edit()
    .with_delete();  // 结果：VIEW_ONLY | EDITABLE | DELETE = 11

// 检查权限
if permission.can_edit() {
    println!("可以编辑内容");
}

// 权限组合操作
let combined = RoomPermission::VIEW_ONLY | RoomPermission::EDITABLE;
assert!(combined.contains(RoomPermission::VIEW_ONLY));
assert!(combined.contains(RoomPermission::EDITABLE));
```
