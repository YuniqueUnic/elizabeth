# Elizabeth 前后端 API 集成计划

同时请你使用 chrome-devtools
进行前端自动化探索和问题修复。同时遇到问题，请你上网查询解决办法

积极使用各种 MCP 工具来辅助你完成相关任务

> 遇到问题请积极修复 保持 DRY, KISS, LISP, 函数化，模块化，可组合性。

后端 service 请使用 `cargo run -p elizabeth-board -- run` 来启动

- 并且后端 service 的配置文件位于：`~/.config/elizabeth/config.yaml`
- 启动了后端 service 后：http://127.0.0.1:4092/api/v1/scalar 这里是 scalar 端口

## 项目背景

- **后端**: Rust + Axum，运行在 `http://localhost:4092`
- **前端**: Next.js 15 + React + TypeScript
- **认证方式**: JWT Token (通过 query 参数 `?token=xxx` 传递)
- **关键发现**: 后端的 `RoomContent` 支持 `ContentType::Text`，可以用于存储消息

## 阶段一：核心功能集成

### 1. 环境配置与类型定义

**文件**: `web/lib/config.ts` (新建)

- 创建 API 配置文件
- 定义后端服务器地址 `http://localhost:4092/api/v1`
- 配置环境变量支持 (开发/生产)

**文件**: `web/lib/types.ts` (更新)

- 对齐后端数据结构
- 添加后端返回的类型定义
- 权限位标志转换工具 (后端使用位标志 1,2,4,8，前端使用字符串数组)
- ContentType 枚举映射 (Text=0, Image=1, File=2, Url=3)

**文件**: `web/lib/utils/api.ts` (新建)

- 创建统一的 API 请求封装
- 实现 token 管理 (localStorage)
- 请求拦截器 (自动添加 token 到 query)
- 错误处理中间件
- 响应格式统一处理

### 2. 房间管理 API 集成

**文件**: `web/api/roomService.ts` (重写)

**API 映射**:

| 功能 | 前端方法 | 后端 API | 说明 |

|------|----------|----------|------|

| 创建房间 | `createRoom(name, password?)` |
`POST /api/v1/rooms/{name}?password=xxx` | - |

| 获取房间信息 | `getRoomDetails(roomId)` | `GET /api/v1/rooms/{name}` |
需要转换权限格式 |

| 更新房间设置 | `updateRoomSettings(roomId, settings)` | 暂不支持 |
使用权限更新 API 代替 |

| 删除房间 | `deleteRoom(roomId, token)` |
`DELETE /api/v1/rooms/{name}?token=xxx` | - |

| 更新权限 | `updateRoomPermissions(roomId, token, permissions)` |
`POST /api/v1/rooms/{name}/permissions?token=xxx` | - |

**关键实现**:

- `getRoomDetails` 需要将后端的 `permission: number` 转换为前端的
  `permissions: string[]`
- `updateRoomSettings` 暂时只支持权限更新，过期时间等后续支持
- 添加房间容量计算逻辑 (current_size / max_size)

### 3. 认证 API 集成

**文件**: `web/api/authService.ts` (新建)

**API 映射**:

| 功能 | 前端方法 | 后端 API | 说明 |

|------|----------|----------|------|

| 获取访问令牌 | `getAccessToken(roomName, password?)` |
`POST /api/v1/rooms/{name}/tokens` | 返回 JWT token |

| 验证令牌 | `validateToken(roomName, token)` |
`POST /api/v1/rooms/{name}/tokens/validate` | - |

| 刷新令牌 | `refreshToken(refreshToken)` | `POST /api/v1/auth/refresh` | - |

| 登出 | `logout(accessToken)` | `POST /api/v1/auth/logout` | 撤销令牌 |

**Token 管理**:

- 在 localStorage 存储当前房间的 token
- 格式：`{ [roomName]: { token, expiresAt, refreshToken? } }`
- 自动刷新机制 (token 过期前 5 分钟)
- 统一的 token 注入到所有 API 请求

### 4. 消息聊天 API 集成 (使用 Content API)

**文件**: `web/api/messageService.ts` (新建)

**核心映射策略**:

- 消息 = `RoomContent` with `content_type: ContentType::Text (0)`
- 消息内容存储在 `text` 字段
- 消息 ID = `content.id`
- 消息时间 = `content.created_at`

**API 映射**:

| 功能 | 前端方法 | 后端 API | 说明 |

|------|----------|----------|------|

| 获取消息列表 | `getMessages(roomName, token)` |
`GET /api/v1/rooms/{name}/contents?token=xxx` | 过滤 content_type=0 |

| 发送消息 | `postMessage(roomName, token, content)` |
`POST /api/v1/rooms/{name}/contents` | 需先 prepare，再上传 |

| 更新消息 | `updateMessage(messageId, content, token)` | 暂不支持 |
前端保留功能，后续实现 |

| 删除消息 | `deleteMessage(roomName, messageId, token)` |
`DELETE /api/v1/rooms/{name}/contents` | 传递 ids 数组 |

**发送消息流程**:

1. 调用 `POST /api/v1/rooms/{name}/contents/prepare` 预留空间

   - 请求：`{ files: [{ name: "message.txt", size: textByteSize, mime: "text/plain" }] }`
   - 获取 `reservation_id`

2. 调用 `POST /api/v1/rooms/{name}/contents?reservation_id=xxx` 上传

   - 使用 FormData 上传，或直接发送 text 内容
   - 需要确认后端是否支持纯文本上传，或需要模拟文件上传

**注意事项**:

- 后端 API 可能需要调整以支持文本内容的直接上传（不通过 multipart/form-data）
- 如果不支持，需要将文本包装成 Blob 上传
- 消息编辑功能暂时不可用，需要后端添加更新 API

### 5. 分享 API 集成

**文件**: `web/api/shareService.ts` (重写)

**API 映射**:

| 功能 | 前端方法 | 后端 API | 说明 |

|------|----------|----------|------|

| 获取分享链接 | `getShareLink(roomName)` | 前端生成 |
`${window.location.origin}/room/${roomName}` |

| 获取二维码 | `getQRCodeImage(roomName)` | 前端生成 | 使用 qrcode 库生成 |

**实现方式**:

- 安装 `qrcode` npm 包
- 前端生成二维码，不依赖后端

### 6. 全局状态管理更新

**文件**: `web/lib/store.ts` (更新)

添加认证状态管理：

```typescript
interface AuthState {
  tokens: Record<string, TokenInfo>; // { [roomName]: { token, expiresAt, refreshToken } }
  currentRoomToken: string | null;
  setRoomToken: (roomName: string, tokenInfo: TokenInfo) => void;
  clearRoomToken: (roomName: string) => void;
  getCurrentToken: () => string | null;
}
```

### 7. Chrome DevTools 自动化测试

**工具**: `chrome-devtools` MCP

**测试场景**:

1. **房间创建与访问流程**

   - 启动前端 (Next.js dev server)
   - 启动后端 (cargo run -p elizabeth-board -- run)
   - 访问首页
   - 输入房间名和密码，创建房间
   - 验证房间创建成功，获得 token
   - 验证 UI 显示正确的房间信息

2. **消息发送与接收流程**

   - 在编辑器中输入消息
   - 点击发送
   - 验证消息出现在列表中
   - 验证消息内容和时间显示正确

3. **权限管理流程**

   - 修改房间权限设置
   - 验证权限更新成功
   - 验证 UI 反映新的权限状态

4. **错误处理测试**

   - 测试无效 token 的处理
   - 测试房间不存在的处理
   - 测试网络错误的处理

**测试文件**: `web/tests/integration.test.ts` (新建)

## 实施步骤

### Step 1: 基础设施搭建

- [ ] 创建 API 配置文件
- [ ] 创建统一的 API 请求封装
- [ ] 更新类型定义
- [ ] 实现权限位标志转换工具

### Step 2: 认证系统

- [ ] 实现 authService (token 获取、验证、刷新)
- [ ] 实现 token 管理机制 (localStorage + store)
- [ ] 实现自动 token 注入中间件

### Step 3: 房间管理

- [ ] 重写 roomService (创建、查询、删除、权限更新)
- [ ] 更新相关组件以使用新的 API
- [ ] 测试房间创建和访问流程

### Step 4: 消息聊天

- [ ] 创建 messageService
- [ ] 实现消息发送流程 (prepare + upload)
- [ ] 实现消息列表获取和过滤
- [ ] 实现消息删除
- [ ] 更新聊天组件以使用新的 API

### Step 5: 分享功能

- [ ] 重写 shareService (前端生成链接和二维码)
- [ ] 安装并集成 qrcode 库
- [ ] 更新分享组件

### Step 6: Chrome DevTools 自动化测试

- [ ] 编写测试脚本
- [ ] 运行完整的集成测试
- [ ] 修复发现的问题

### Step 7: 错误处理和优化

- [ ] 完善错误处理
- [ ] 添加加载状态
- [ ] 优化用户体验
- [ ] 更新文档

## 技术要点

### 1. Token 认证流程

```
1. 用户输入房间名 + 密码
   ↓
2. POST /api/v1/rooms/{name}/tokens { password }
   ↓
3. 获取 { token, expires_at, refresh_token }
   ↓
4. 存储到 localStorage 和 store
   ↓
5. 所有后续请求自动添加 ?token=xxx
```

### 2. 消息发送流程

```
1. 用户输入消息内容
   ↓
2. POST /api/v1/rooms/{name}/contents/prepare
   { files: [{ name: "message.txt", size, mime: "text/plain" }] }
   ↓
3. 获取 reservation_id
   ↓
4. POST /api/v1/rooms/{name}/contents?reservation_id=xxx
   FormData: { file: new Blob([text], { type: "text/plain" }) }
   ↓
5. 获取新创建的 RoomContent (content_type=0)
   ↓
6. 更新前端消息列表
```

### 3. 权限转换

```typescript
// 后端 -> 前端
function parsePermissions(bits: number): string[] {
  const perms: string[] = [];
  if (bits & 1) perms.push("read");
  if (bits & 2) perms.push("edit");
  if (bits & 4) perms.push("share");
  if (bits & 8) perms.push("delete");
  return perms;
}

// 前端 -> 后端
function encodePermissions(perms: string[]): number {
  let bits = 0;
  if (perms.includes("read")) bits |= 1;
  if (perms.includes("edit")) bits |= 2;
  if (perms.includes("share")) bits |= 4;
  if (perms.includes("delete")) bits |= 8;
  return bits;
}
```

## 潜在问题和解决方案

### 问题 1: 消息编辑功能

**现状**: 前端有消息编辑功能，但后端不支持更新 content

**解决方案**:

- 短期：前端禁用编辑按钮
- 长期：后端添加 `PUT /api/v1/rooms/{name}/contents/{id}` API

### 问题 2: 文本内容上传方式

**现状**: 后端 upload_contents 可能只接受文件上传

**解决方案**:

- 将文本内容包装成 Blob: `new Blob([text], { type: 'text/plain' })`
- 或确认后端是否支持纯 JSON 上传文本内容

### 问题 3: 房间设置更新

**现状**: 后端没有统一的房间设置更新 API

**解决方案**:

- 短期：只实现权限更新功能
- 长期：确认是否需要添加房间元数据更新 API

## 验收标准

- [ ] 用户可以创建房间并设置密码
- [ ] 用户可以通过密码获取访问令牌
- [ ] 用户可以在房间中发送和接收消息
- [ ] 用户可以删除消息
- [ ] 用户可以管理房间权限
- [ ] 用户可以删除房间
- [ ] 所有 API 调用都使用真实的后端接口
- [ ] Token 自动管理和刷新正常工作
- [ ] 错误处理完善，用户体验良好
- [ ] Chrome DevTools 自动化测试全部通过

## 文档更新

计划完成后需要更新以下文档：

- `web/docs/FRONTEND_DOCUMENTATION.md` - 更新 API 集成章节
- `docs/api-documentation.md` - 补充实际使用案例
- `docs/current-progress-docs.md` - 记录集成进度
