# Elizabeth

Elizabeth 是一个现代化的、以房间为中心的文件分享与协作平台，采用 Rust + Next.js
技术栈构建，提供简单、安全、高效的文件共享解决方案。

## 项目架构概览

Elizabeth 采用前后端分离架构，后端基于 Rust 的 Axum 框架提供 RESTful API 和
WebSocket 服务，前端基于 Next.js 16 + React 19 构建现代化用户界面。

```
┌─────────────────────────────────────────────────────────────┐
│                        Elizabeth                            │
│                   文件分享与协作平台                        │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        │                                       │
┌───────▼────────┐                    ┌────────▼─────────┐
│   Frontend     │                    │     Backend      │
│   Next.js 16   │◄───────────────────►     Rust         │
│   React 19     │    HTTP/WebSocket  │     Axum         │
└────────────────┘                    └──────────────────┘
                                              │
                                      ┌───────┴────────┐
                                      │                │
                                ┌─────▼──────┐  ┌─────▼──────┐
                                │  SQLite/   │  │   File     │
                                │  PostgreSQL│  │  Storage   │
                                └────────────┘  └────────────┘
```

## 核心理念

**"以房间为中心" (Room-centric)**，而不是传统的"以用户为中心"。

### 核心价值

- **无用户系统**: 无需注册登录，通过房间进行身份验证
- **房间即身份**: 用户通过进入一个"房间"来完成身份验证
- **安全优先**: 提供全链路加密支持，包括传输安全 (TLS) 和存储安全
- **临时性与可控性**: 所有共享都发生在"房间"内，房间本身是临时的，可以被精细控制

## 核心特性

### 房间管理

- 创建房间：支持设置密码、过期时间、访问次数限制
- 房间权限：灵活的权限控制（编辑、下载、预览）
- 房间设置：可配置房间大小限制、进入次数限制
- Token 认证：基于 JWT 的房间访问令牌系统
- 自动过期：支持房间自动过期和清理

### 内容管理

- 文件上传：支持单文件和批量文件上传
- 分块上传：支持大文件分块上传，断点续传
- 文件下载：支持单文件和批量下载
- 内容预览：支持多种文件类型预览
- 内容编辑：支持文本内容在线编辑
- URL 分享：支持通过 URL 分享内容

### 协作功能

- 实时协作：支持 Markdown 的消息系统
- WebSocket 实时通信：房间内实时事件推送
  - 内容创建/更新/删除实时同步
  - 用户加入/离开通知
  - 自动重连机制
  - 心跳检测
- 消息编辑：可编辑历史消息
- 文件共享：房间内成员共享文件
- 权限管理：细粒度的权限控制

### 用户体验

- 响应式设计：适配各种屏幕尺寸
- 主题切换：支持暗色/亮色/跟随系统三种主题模式
- 拖拽上传：支持拖拽文件上传
- 批量操作：支持批量下载、删除等操作

## 技术栈

### 后端 (Rust)

- **Rust 1.90+**: 核心编程语言
- **Axum 0.8.6**: 异步 Web 框架
- **SQLx 0.8**: 异步 SQL 工具包，支持编译时查询检查
- **SQLite/PostgreSQL**: 数据库支持
- **Tokio**: 异步运行时
- **Serde**: 序列化/反序列化
- **Utoipa**: OpenAPI 文档自动生成
- **tokio-tungstenite**: WebSocket 支持
- **ts-rs**: Rust 到 TypeScript 类型自动生成
- **JWT**: 基于 JSON Web Token 的认证

### 前端 (Next.js)

- **Next.js 16**: React 框架，支持 App Router
- **React 19**: UI 库
- **TypeScript**: 类型安全
- **shadcn/ui**: UI 组件库
- **Tailwind CSS v4**: 样式框架
- **Zustand**: 状态管理
- **TanStack Query**: 数据请求和缓存
- **react-markdown**: Markdown 渲染
- **react-dropzone**: 文件拖拽上传

## 项目结构

### 代码库组织

```
elizabeth/
├── crates/                        # Rust 后端 Workspace
│   ├── board/                     # 主应用程序
│   │   ├── src/
│   │   │   ├── handlers/          # HTTP 请求处理器
│   │   │   │   ├── rooms.rs       # 房间管理 API
│   │   │   │   ├── content.rs     # 内容管理 API
│   │   │   │   ├── chunked_upload.rs # 分块上传 API
│   │   │   │   ├── auth.rs        # 认证 API
│   │   │   │   ├── refresh_token.rs # 令牌刷新 API
│   │   │   │   └── admin.rs       # 管理员 API
│   │   │   ├── models/            # （已移至 board-protocol）
│   │   │   ├── repository/        # 数据访问层
│   │   │   │   ├── room_repository.rs
│   │   │   │   ├── content_repository.rs
│   │   │   │   ├── token_repository.rs
│   │   │   │   ├── refresh_token_repository.rs
│   │   │   │   ├── chunk_upload_repository.rs
│   │   │   │   └── upload_reservation_repository.rs
│   │   │   ├── route/             # 路由定义
│   │   │   │   ├── room.rs        # 房间路由
│   │   │   │   ├── auth.rs        # 认证路由
│   │   │   │   ├── ws.rs          # WebSocket 路由
│   │   │   │   ├── admin.rs       # 管理路由
│   │   │   │   └── status.rs      # 状态路由
│   │   │   ├── services/          # 业务逻辑服务
│   │   │   │   ├── auth_service.rs    # 认证服务
│   │   │   │   ├── token_service.rs   # Token 管理
│   │   │   │   └── room_gc_service.rs # 房间清理
│   │   │   ├── middleware/        # 中间件
│   │   │   │   ├── cors.rs        # CORS 配置
│   │   │   │   ├── request_id.rs  # 请求 ID 追踪
│   │   │   │   └── security.rs    # 安全头设置
│   │   │   ├── websocket/         # WebSocket 模块
│   │   │   │   ├── server.rs      # WebSocket 服务器
│   │   │   │   ├── handler.rs     # 消息处理器
│   │   │   │   ├── connection.rs  # 连接管理
│   │   │   │   └── types.rs       # WebSocket 类型
│   │   │   ├── validation/        # 数据验证
│   │   │   ├── storage/           # 存储抽象
│   │   │   ├── config.rs          # 配置管理
│   │   │   ├── state.rs           # 应用状态
│   │   │   ├── errors.rs          # 错误定义
│   │   │   └── lib.rs             # 库入口
│   │   ├── migrations/            # SQLite 数据库迁移
│   │   ├── migrations_pg/         # PostgreSQL 数据库迁移
│   │   └── tests/                 # 集成测试
│   ├── board-protocol/            # 共享协议和模型
│   │   ├── src/
│   │   │   ├── models/            # 数据模型定义
│   │   │   │   ├── room/          # 房间相关模型
│   │   │   │   │   ├── mod.rs     # Room 核心模型
│   │   │   │   │   ├── content.rs # RoomContent 模型
│   │   │   │   │   ├── token.rs   # RoomToken 模型
│   │   │   │   │   ├── refresh_token.rs # RefreshToken 模型
│   │   │   │   │   ├── chunk_upload.rs # ChunkUpload 模型
│   │   │   │   │   ├── upload_reservation.rs # UploadReservation 模型
│   │   │   │   │   └── permission.rs # Permission 模型
│   │   │   │   └── permission.rs  # 权限枚举
│   │   │   ├── dto/               # 数据传输对象
│   │   │   │   ├── room.rs        # 房间 DTO
│   │   │   │   ├── content.rs     # 内容 DTO
│   │   │   │   ├── token.rs       # Token DTO
│   │   │   │   └── upload.rs      # 上传 DTO
│   │   │   └── constants/         # 常量定义
│   │   └── bindings/              # TypeScript 类型绑定
│   ├── configrs/                  # 配置管理库
│   │   └── src/
│   │       ├── configs/           # 配置结构
│   │       │   └── app.rs         # 应用配置
│   │       └── lib.rs             # 配置加载器
│   └── logrs/                     # 日志管理库
│       └── src/
│           └── lib.rs             # 日志配置
├── web/                           # Next.js 前端
│   ├── app/                       # App Router
│   │   ├── layout.tsx             # 根布局
│   │   ├── page.tsx               # 主页面
│   │   └── globals.css            # 全局样式
│   ├── components/                # React 组件
│   │   ├── layout/                # 布局组件
│   │   │   ├── top-bar.tsx        # 顶部栏
│   │   │   ├── left-sidebar.tsx   # 左侧边栏（房间控制）
│   │   │   ├── middle-column.tsx  # 中间栏（聊天）
│   │   │   └── right-sidebar.tsx  # 右侧边栏（文件管理）
│   │   ├── chat/                  # 聊天组件
│   │   ├── files/                 # 文件管理组件
│   │   ├── room/                  # 房间设置组件
│   │   └── ui/                    # 基础 UI 组件（shadcn/ui）
│   ├── lib/                       # 工具库
│   │   ├── api/                   # API 服务层
│   │   │   ├── roomService.ts     # 房间 API
│   │   │   ├── fileService.ts     # 文件 API
│   │   │   └── shareService.ts    # 分享 API
│   │   ├── store.ts               # Zustand 状态管理
│   │   ├── types.ts               # TypeScript 类型
│   │   └── hooks/                 # 自定义 Hooks
│   ├── types/                     # 类型定义
│   │   └── generated/             # 自动生成的类型
│   └── public/                    # 静态资源
├── migrations/                    # SQLite 数据库迁移
├── migrations_pg/                 # PostgreSQL 数据库迁移
├── docs/                          # 项目文档
│   ├── DEPLOYMENT.md              # 部署文档
│   ├── DOCKER_QUICK_START.md      # Docker 快速开始
│   ├── IMPLEMENTATION_GUIDE.md    # 实现指南
│   ├── WEBSOCKET_GUIDE.md         # WebSocket 指南
│   └── implementation/            # 实现文档
├── scripts/                       # 部署脚本
│   ├── deploy.sh                  # 部署脚本
│   ├── backup.sh                  # 备份脚本
│   └── restore.sh                 # 恢复脚本
├── docker-compose.yml             # Docker Compose 配置
├── Dockerfile.backend             # 后端 Docker 镜像
├── Dockerfile.frontend            # 前端 Docker 镜像
├── justfile                       # Just 任务定义
├── Cargo.toml                     # Rust Workspace 配置
└── Cargo.lock                     # Rust 依赖锁定
```

## 后端架构详解

### 分层架构

Elizabeth 后端采用清晰的分层架构，从外到内依次为：

```
┌──────────────────────────────────────────────────────────────┐
│                    Route Layer (路由层)                      │
│  职责：API 端点定义、中间件配置、OpenAPI 文档生成            │
│  文件：route/{room,auth,ws,admin,status}.rs                  │
└────────────────────────┬─────────────────────────────────────┘
                         │ axum::Router
┌────────────────────────▼─────────────────────────────────────┐
│                   Handler Layer (处理层)                     │
│  职责：HTTP 请求解析、参数验证、响应构建、错误处理           │
│  文件：handlers/{rooms,content,chunked_upload,auth,admin}.rs │
└────────────────────────┬─────────────────────────────────────┘
                         │ AppState + Repository
┌────────────────────────▼─────────────────────────────────────┐
│                   Service Layer (服务层)                     │
│  职责：业务逻辑实现、Token 管理、权限验证、房间清理          │
│  文件：services/{auth_service,token_service,room_gc}.rs      │
└────────────────────────┬─────────────────────────────────────┘
                         │ Repository Pattern
┌────────────────────────▼─────────────────────────────────────┐
│                 Repository Layer (仓库层)                    │
│  职责：数据访问抽象、SQL 查询、事务管理                      │
│  文件：repository/{room,content,token,refresh_token,         │
│         chunk_upload,upload_reservation}_repository.rs       │
└────────────────────────┬─────────────────────────────────────┘
                         │ SQLx
┌────────────────────────▼─────────────────────────────────────┐
│                   Database (数据库层)                        │
│  SQLite (默认) / PostgreSQL (可选)                           │
└──────────────────────────────────────────────────────────────┘
```

### 核心模块说明

#### 1. Models (数据模型层)

位于 `board-protocol` crate，定义核心数据结构：

- **Room**: 房间模型，包含房间的所有属性和状态
- **RoomContent**: 房间内容模型，支持文件、文本、URL 三种类型
- **RoomToken**: 房间访问令牌
- **RoomRefreshToken**: 刷新令牌
- **RoomChunkUpload**: 分块上传记录
- **RoomUploadReservation**: 上传预留记录
- **RoomPermission**: 权限位标志

#### 2. Repository (数据访问层)

使用 Repository 模式封装数据库操作：

```rust
// RoomRepository 核心方法
pub async fn create(&self, room: &Room) -> Result<Room>
pub async fn find_by_name(&self, name: &str) -> Result<Option<Room>>
pub async fn update(&self, room: &Room) -> Result<Room>
pub async fn delete(&self, id: i64) -> Result<()>
pub async fn exists(&self, name: &str) -> Result<bool>

// ContentRepository 核心方法
pub async fn create(&self, content: &RoomContent) -> Result<RoomContent>
pub async fn find_by_id(&self, id: i64) -> Result<Option<RoomContent>>
pub async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomContent>>
pub async fn update(&self, content: &RoomContent) -> Result<RoomContent>
pub async fn delete(&self, id: i64) -> Result<()>
```

#### 3. Services (业务逻辑层)

- **AuthService**: 认证服务，负责 Token 验证、权限检查
- **TokenService**: Token 生成和解析
- **RoomGcService**: 房间垃圾回收，自动清理过期房间

#### 4. Handlers (请求处理层)

每个 handler 负责一类 API 端点：

```rust
// rooms.rs - 房间管理
pub async fn create(...)      // POST   /rooms/{name}
pub async fn find(...)        // GET    /rooms/{name}
pub async fn delete(...)      // DELETE /rooms/{name}
pub async fn update_permissions(...) // PUT /rooms/{name}/permissions
pub async fn issue_token(...) // POST   /rooms/{name}/tokens

// content.rs - 内容管理
pub async fn prepare_upload(...)    // POST   /rooms/{name}/contents/prepare
pub async fn upload_contents(...)   // POST   /rooms/{name}/contents
pub async fn list_contents(...)     // GET    /rooms/{name}/contents
pub async fn download_content(...)  // GET    /rooms/{name}/contents/{id}/download
pub async fn delete_content(...)    // DELETE /rooms/{name}/contents

// chunked_upload.rs - 分块上传
pub async fn prepare_chunked_upload(...)  // POST /rooms/{name}/chunked-uploads/prepare
pub async fn upload_chunk(...)            // POST /rooms/{name}/chunked-uploads/{id}/chunks/{index}
pub async fn merge_chunks(...)            // POST /rooms/{name}/chunked-uploads/{id}/merge
pub async fn get_upload_status(...)       // GET  /rooms/{name}/chunked-uploads/{id}/status
```

#### 5. WebSocket (实时通信)

WebSocket 模块提供房间级别的实时事件推送：

```
┌─────────────────────────────────────────────────────────┐
│                   WebSocket Server                      │
│  入口：/api/v1/ws                                       │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────▼───────────┐
         │  Connection Manager   │
         │  管理所有活跃连接      │
         └───────────┬───────────┘
                     │
         ┌───────────▼───────────┐
         │   Message Handler     │
         │  处理业务消息          │
         └───────────────────────┘

消息类型：
- CONNECT: 连接建立（需要提供 room_name 和 token）
- CONNECT_ACK: 连接确认
- PING/PONG: 心跳检测
- CONTENT_CREATED: 内容创建事件
- CONTENT_UPDATED: 内容更新事件
- CONTENT_DELETED: 内容删除事件
- USER_JOINED: 用户加入房间
- USER_LEFT: 用户离开房间
- ERROR: 错误消息
```

### 数据流程图

#### 房间创建流程

```
┌──────┐                                    ┌─────────────┐
│Client│                                    │    Server   │
└──┬───┘                                    └──────┬──────┘
   │                                               │
   │  POST /api/v1/rooms/{name}?password=xxx       │
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 验证房间名称     │
   │                                    │  2. 验证密码格式     │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  3. 检查房间是否存在 │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  4. 创建 Room 对象   │
   │                                    │  5. 应用默认配置     │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  6. 写入数据库       │
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  { "id": 1, "name": "xxx", ... }               │
   │                                                │
```

#### Token 签发与验证流程

```
┌──────┐                                    ┌─────────────┐
│Client│                                    │    Server   │
└──┬───┘                                    └──────┬──────┘
   │                                                │
   │  POST /api/v1/rooms/{name}/tokens              │
   │  Body: { "password": "xxx" }                   │               # pragma: allowlist secret
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 查找房间         │
   │                                    │  2. 验证密码         │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  3. 检查房间状态     │
   │                                    │  - is_expired()      │
   │                                    │  - can_enter()       │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  4. 递增访问计数      │
   │                                    │  current_times_entered++│
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  5. 生成 JWT Token   │
   │                                    │  - access_token      │
   │                                    │  - refresh_token (可选)│
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  {                                             │
   │    "access_token": "eyJ...",                   │
   │    "refresh_token": "eyJ...",                  │
   │    "expires_in": 7200                          │
   │  }                                             │
   │                                                │

后续请求携带 Token：

   │  GET /api/v1/rooms/{name}/contents?token=eyJ...│
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 解码 JWT Token   │
   │                                    │  2. 验证签名         │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  3. 检查黑名单       │
   │                                    │  4. 验证过期时间     │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  5. 验证房间 ID      │
   │                                    │  6. 检查权限         │
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  [ content list ]                              │
```

#### 文件上传流程（分块上传）

```
┌──────┐                                     ┌─────────────┐
│Client│                                     │    Server   │
└──┬───┘                                     └──────┬──────┘
   │                                                │
   │  第一步：准备上传                              │
   │  POST /api/v1/rooms/{name}/chunked-uploads/prepare│
   │  Body: {                                       │
   │    "files": [{                                 │
   │      "file_name": "large.zip",                 │
   │      "size": 104857600,                        │
   │      "total_chunks": 10,                       │
   │      "chunk_size": 10485760                    │
   │    }]                                          │
   │  }                                             │
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 验证房间空间     │
   │                                    │  2. 创建上传预留     │
   │                                    │  3. 生成 upload_id   │
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  {                                             │
   │    "upload_id": "uuid-xxx",                    │
   │    "files": [ { "upload_token": "uuid-yyy" } ] │
   │  }                                             │
   │                                                │
   │  第二步：上传分块（重复 10 次）                │
   │  POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/chunks/0│
   │  Body: [binary chunk data]                     │
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 验证分块索引     │
   │                                    │  2. 计算哈希         │
   │                                    │  3. 保存临时文件     │
   │                                    │  4. 更新上传记录     │
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  { "chunk_index": 0, "status": "completed" }   │
   │                                                │
   │  ... (重复上传其他分块) ...                    │
   │                                                │
   │  第三步：合并文件                              │
   │  POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/merge│
   │  Body: { "file_hash": "sha256-xxx" }           │
   ├───────────────────────────────────────────────►
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  1. 检查所有分块完成 │
   │                                    │  2. 按顺序合并文件   │
   │                                    │  3. 验证文件哈希     │
   │                                    └───────────┬──────────┘
   │                                                │
   │                                    ┌───────────▼──────────┐
   │                                    │  4. 创建 RoomContent │
   │                                    │  5. 清理临时文件     │
   │                                    │  6. 更新房间大小     │
   │                                    └───────────┬──────────┘
   │                                                │
   │  ◄─────────────────────────────────────────────┤
   │  {                                             │
   │    "content_id": 123,                          │
   │    "file_name": "large.zip",                   │
   │    "size": 104857600                           │
   │  }                                             │
   │                                                │
```

## 前端架构详解

### 组件化设计

```
┌──────────────────────────────────────────────────────────┐
│                      App Layout                          │
│  app/layout.tsx - 根布局，提供全局状态和主题             │
└────────────────────┬─────────────────────────────────────┘
                     │
         ┌───────────▼───────────┐
         │     Page Container    │
         │   app/page.tsx        │
         └───────────┬───────────┘
                     │
     ┌───────────────┼───────────────┐
     │               │               │
┌────▼────┐    ┌────▼────┐    ┌────▼────┐
│  Left   │    │ Middle  │     │  Right  │
│ Sidebar │    │ Column  │     │ Sidebar │
│ (Room)  │    │ (Chat)  │     │ (Files) │
└─────────┘    └─────────┘     └─────────┘
```

## 快速开始

### Docker 部署（推荐）

使用 Docker 是最简单的部署方式，无需安装 Rust 和 Node.js 环境。

#### 前置要求

- Docker 20.10+
- Docker Compose 2.0+
- Just (可选，用于简化命令)

#### 一键部署

```bash
# 1. 克隆仓库
git clone https://github.com/yuniqueunic/elizabeth.git
cd elizabeth

# 2. 初始化配置
just docker-init
# 或者: cp .env.docker .env

# 3. 编辑 .env 文件，设置 JWT_SECRET
openssl rand -base64 48  # 生成安全密钥
# 将生成的密钥设置到 .env 文件中的 JWT_SECRET

# 4. 一键部署
just docker-deploy
# 或者: ./scripts/deploy.sh

# 5. 访问应用
# 前端: http://localhost:4001
# 后端 API（经由网关转发）: http://localhost:4001/api/v1
# API 文档（经由网关转发）: http://localhost:4001/api/v1/scalar
```

#### 常用 Docker 命令

```bash
# 使用 Just (推荐)
just docker-status          # 查看服务状态
just docker-logs            # 查看日志
just docker-logs backend    # 查看后端日志
just docker-backup          # 备份数据
just docker-restart         # 重启服务
just docker-down            # 停止服务

# 或使用 Docker Compose
docker-compose ps           # 查看状态
docker-compose logs -f      # 查看日志
docker-compose restart      # 重启服务
docker-compose down         # 停止服务
```

详细的 Docker 部署文档请参考：

- [Docker 快速开始指南](./docs/DOCKER_QUICK_START.md)
- [完整部署文档](./docs/DEPLOYMENT.md)

### 本地开发环境

如果需要进行开发，可以在本地搭建开发环境。

#### 环境要求

- Rust 1.90+
- Node.js 20+
- pnpm 8+
- Git
- SQLite 3
- Just (可选)

#### 安装和构建

1. **克隆仓库**

   ```bash
   git clone https://github.com/yuniqueunic/elizabeth.git
   cd elizabeth
   ```

2. **后端设置**

   ```bash
   # 初始化数据库
   just migrate
   # 或者：cargo sqlx migrate run

   # 运行后端
   just run
   # 或者：cargo run -p elizabeth-board -- run
   ```

3. **前端设置**

   ```bash
   cd web
   pnpm install
   pnpm dev --port 4001
   ```

4. **访问应用**
   - 前端：http://localhost:4001
   - 后端 API：http://localhost:4092/api/v1（本地开发） /
     http://localhost:4001/api/v1（Docker）
   - API 文档：http://localhost:4092/api/v1/scalar（本地开发） /
     http://localhost:4001/api/v1/scalar（Docker）

#### 开发环境设置

1. **运行测试**

   ```bash
   cargo test
   ```

2. **检查代码格式**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

## API 文档

项目提供完整的 RESTful API，所有 API 都有完整的 OpenAPI 文档。

### API 基础信息

- **API 前缀**: `/api/v1`
- **默认端口**: `4092`（本地运行后端）
- **Docker 访问入口**: `http://localhost:4001/api/v1`（后端不对宿主机暴露）
- **API 文档**: `http://localhost:4092/api/v1/scalar`（本地） /
  `http://localhost:4001/api/v1/scalar`（Docker）

### 主要 API 端点

#### 状态检查

- `GET /api/v1/health` - 健康检查
- `GET /api/v1/status` - 服务状态

#### 房间管理

- `POST /api/v1/rooms/{name}` - 创建房间
- `GET /api/v1/rooms/{name}` - 查询房间
- `DELETE /api/v1/rooms/{name}` - 删除房间
- `PUT /api/v1/rooms/{name}/permissions` - 更新房间权限
- `PUT /api/v1/rooms/{name}/settings` - 更新房间设置

#### Token 管理

- `POST /api/v1/rooms/{name}/tokens` - 签发房间 Token
- `GET /api/v1/rooms/{name}/tokens` - 列出房间 Token
- `POST /api/v1/rooms/{name}/tokens/validate` - 验证 Token
- `DELETE /api/v1/rooms/{name}/tokens/{jti}` - 撤销 Token

#### 内容管理

- `GET /api/v1/rooms/{name}/contents` - 列出房间内容
- `POST /api/v1/rooms/{name}/contents/prepare` - 准备上传
- `POST /api/v1/rooms/{name}/contents` - 上传内容
- `PUT /api/v1/rooms/{name}/contents/{id}` - 更新内容
- `DELETE /api/v1/rooms/{name}/contents` - 删除内容
- `GET /api/v1/rooms/{name}/contents/{id}/download` - 下载内容

#### 分块上传

- `POST /api/v1/rooms/{name}/chunked-uploads/prepare` - 准备分块上传
- `POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/chunks/{chunk_index}` -
  上传分块
- `GET /api/v1/rooms/{name}/chunked-uploads/{upload_id}/status` - 查询上传状态
- `POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/merge` - 合并文件

#### 刷新令牌

- `POST /api/v1/refresh-token` - 刷新访问令牌
- `DELETE /api/v1/refresh-token` - 撤销刷新令牌
- `DELETE /api/v1/refresh-token/cleanup` - 清理过期令牌

#### WebSocket

- `WS /api/v1/ws` - WebSocket 连接端点
  - 支持房间级别的事件订阅
  - 实时推送房间内容变更
  - 用户在线状态通知

详细的 WebSocket 使用指南请参考：[WebSocket 指南](./docs/WEBSOCKET_GUIDE.md)

### OpenAPI 文档

启动服务后，可以通过以下地址访问交互式 API 文档：

- **Scalar UI**: `http://localhost:4092/api/v1/scalar`（本地） /
  `http://localhost:4001/api/v1/scalar`（Docker）

### 使用示例

#### 1. 创建房间

```bash
# Base URL:
# - Local dev: http://localhost:4092/api/v1
# - Docker:    http://localhost:4001/api/v1
API_BASE="http://localhost:4092/api/v1"

# 创建带密码的房间
curl -X POST "${API_BASE}/rooms/myroom?password=secret123"
```

#### 2. 签发 Token

```bash
# Base URL:
# - Local dev: http://localhost:4092/api/v1
# - Docker:    http://localhost:4001/api/v1
API_BASE="http://localhost:4092/api/v1"

# 使用密码签发 Token
curl -X POST "${API_BASE}/rooms/myroom/tokens" \
  -H "Content-Type: application/json" \
  -d '{"password": "secret123", "with_refresh_token": true}' # pragma: allowlist secret
```

#### 3. 上传文件

```bash
# Base URL:
# - Local dev: http://localhost:4092/api/v1
# - Docker:    http://localhost:4001/api/v1
API_BASE="http://localhost:4092/api/v1"

# 准备上传
curl -X POST "${API_BASE}/rooms/myroom/contents/prepare" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"files": [{"file_name": "test.txt", "size": 1024, "mime_type": "text/plain"}]}'

# 上传文件
curl -X POST "${API_BASE}/rooms/myroom/contents?token=YOUR_TOKEN" \
  -F "files=@test.txt"
```

#### 4. 列出房间内容

```bash
# Base URL:
# - Local dev: http://localhost:4092/api/v1
# - Docker:    http://localhost:4001/api/v1
API_BASE="http://localhost:4092/api/v1"

curl -X GET "${API_BASE}/rooms/myroom/contents?token=YOUR_TOKEN"
```

#### 5. 下载文件

```bash
# Base URL:
# - Local dev: http://localhost:4092/api/v1
# - Docker:    http://localhost:4001/api/v1
API_BASE="http://localhost:4092/api/v1"

curl -X GET "${API_BASE}/rooms/myroom/contents/1/download?token=YOUR_TOKEN" \
  -o downloaded_file.txt
```

## 配置管理

### 后端配置

后端使用分层配置系统，支持多种配置来源：

1. **默认配置**: 代码中的默认值
2. **配置文件**: `~/.config/elizabeth/config.yaml`
3. **环境变量**: `ELIZABETH__APP__*` 前缀

配置示例：

```yaml
app:
  server:
    addr: 127.0.0.1
    port: 4092
  logging:
    level: info
  database:
    url: sqlite:app.db
    max_connections: 20
  storage:
    root: ./storage/rooms
  jwt:
    secret: your-secret-key-min-32-chars
    ttl_seconds: 7200
    refresh_ttl_seconds: 604800
  room:
    max_size: 52428800 # 50MB
    max_times_entered: 100
  middleware:
    cors:
      enabled: true
      allowed_origins: "*"
    rate_limit:
      enabled: false
      per_second: 10
```

### 前端配置

前端使用环境变量配置：

```bash
# .env.local
NEXT_PUBLIC_API_URL=/api/v1
INTERNAL_API_URL=http://localhost:4092/api/v1
NEXT_PUBLIC_APP_URL=http://localhost:4001
```

### Docker 配置

Docker 部署时，所有配置通过 `.env` 文件管理。详见
[Docker 快速开始](./docs/DOCKER_QUICK_START.md)。

#### 避免 HTTPS 混合内容

- 浏览器侧 API 地址请使用相对路径：`NEXT_PUBLIC_API_URL=/api/v1`
- Next.js
  服务端转发内部地址：`INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1`
- 重新构建前端镜像确保打包产物不再包含 `http://`
  明文后端地址：`docker compose build --no-cache frontend && docker compose up -d frontend gateway`
- 参考文档：[docs/https-proxy-unification.md](./docs/https-proxy-unification.md)

#### 数据库选择（SQLite / PostgreSQL）

- 默认 SQLite：`DATABASE_URL=sqlite:app.db` 将使用 `migrations/` 目录。
- PostgreSQL：将 `DATABASE_URL` 设置为 `postgres://...` 时自动切换为
  `migrations_pg/` 目录，并使用 `sqlx` Postgres 驱动。
- 后端镜像需包含 postgres
  特性（已启用）。切换数据库后请重新构建后端镜像并运行迁移。

## 测试

### 后端测试

```bash
# 运行所有测试
cargo test

# 运行特定测试模块
cargo test room_repository_tests
cargo test api_integration_tests

# 运行测试并显示输出
cargo test -- --nocapture

# 使用 just 运行测试
just test
```

### 前端测试

```bash
cd web

# 运行测试
pnpm test

# 运行测试并监听变化
pnpm test:watch
```

### 测试覆盖

#### 后端

- Repository 单元测试
- 数据库操作测试
- API 集成测试
- 业务逻辑测试

#### 前端

- 组件测试
- API 服务测试
- 状态管理测试

## 发布系统

Elizabeth 项目使用 [release-plz](https://release-plz.ieni.dev/)
实现自动化版本发布流程。该系统能够：

- 根据 Conventional Commits 自动确定版本号
- 自动生成和维护 changelog
- 创建 GitHub Release
- 与 GitHub Actions 无缝集成

### 发布流程

1. **日常开发**: 在功能分支上进行开发，使用 Conventional Commits 格式提交
2. **合并代码**: 将功能分支合并到 main 分支
3. **自动创建发布 PR**: GitHub Actions 自动创建包含版本更新和 changelog 的 PR
4. **审核发布**: 审核自动生成的 PR，确认无误后合并
5. **自动发布**: 合并 PR 后自动执行发布流程，创建 git 标签

### Conventional Commits 规范

项目遵循 Conventional Commits 规范，支持的提交类型包括：

- `feat`: 新功能
- `fix`: 修复 bug
- `perf`: 性能优化
- `refactor`: 代码重构
- `docs`: 文档更新
- `style`: 代码格式调整
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动
- `build`: 构建系统或依赖变更
- `ci`: CI 配置文件和脚本的变更

#### 提交示例

```bash
# 新功能
git commit -m "feat(auth): add user authentication"

# 修复 bug
git commit -m "fix(login): resolve token expiration issue"

# 破坏性更改
git commit -m "feat(api)!: change user endpoint response format"
```

详细的发布系统配置和使用方法请参考
[`docs/release-plz.md`](./docs/great-blog/release-plz.md)。

## 开发指南

### 代码规范

#### 后端 (Rust)

```bash
# 格式化代码
cargo fmt

# 代码检查
cargo clippy -- -D warnings

# 使用 just
just fmt
just clippy
```

#### 前端 (TypeScript)

```bash
cd web

# 格式化代码
pnpm format

# 代码检查
pnpm lint

# 类型检查
pnpm type-check
```

### 分支策略

- `main`: 主分支，保持稳定状态
- `feature/*`: 功能分支，用于开发新功能
- `fix/*`: 修复分支，用于修复 bug
- `release-plz-*`: 发布分支，由 release-plz 自动创建

### 提交流程

1. 从 main 分支创建功能分支
2. 在功能分支上进行开发和测试
3. 提交代码，使用 Conventional Commits 格式
4. 创建 Pull Request 到 main 分支
5. 代码审查通过后合并
6. 等待自动创建发布 PR

### 常用命令

使用 `just` 简化开发流程：

```bash
# 查看所有可用命令
just --list

# 后端开发
just run              # 运行后端服务
just test             # 运行测试
just migrate          # 运行数据库迁移
just reset-db         # 重置数据库

# 前端开发
just web-dev          # 运行前端开发服务器
just web-build        # 构建前端

# Docker 部署
just docker-deploy    # 一键部署
just docker-status    # 查看状态
just docker-logs      # 查看日志
just docker-backup    # 备份数据
```

## 贡献指南

我们欢迎所有形式的贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 贡献类型

- Bug 修复
- 新功能开发
- 文档改进
- 代码优化和重构
- 性能优化
- 测试覆盖

### 开发环境设置

```bash
# 1. Fork 并克隆项目
git clone https://github.com/yuniqueunic/elizabeth.git
cd elizabeth

# 2. 安装依赖
# 后端
cargo build

# 前端
cd web
pnpm install

# 3. 运行开发环境
# 后端
just run

# 前端
just web-dev

# 4. 运行测试
just test
```

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 致谢

感谢所有为 Elizabeth 项目做出贡献的开发者和社区成员！

### 主要技术栈

#### 后端

- [Axum](https://github.com/tokio-rs/axum) - 异步 Web 框架
- [SQLx](https://github.com/launchbadge/sqlx) - 异步 SQL 工具包
- [Tokio](https://tokio.rs/) - 异步运行时
- [Utoipa](https://github.com/juhaku/utoipa) - OpenAPI 文档生成

#### 前端

- [Next.js](https://nextjs.org/) - React 框架
- [shadcn/ui](https://ui.shadcn.com/) - UI 组件库
- [Tailwind CSS](https://tailwindcss.com/) - CSS 框架
- [Zustand](https://github.com/pmndrs/zustand) - 状态管理
- [TanStack Query](https://tanstack.com/query) - 数据请求

#### 开发工具

- [Just](https://github.com/casey/just) - 命令运行器
- [Docker](https://www.docker.com/) - 容器化部署
- [release-plz](https://release-plz.ieni.dev/) - 自动化发布

### 相关项目

- [microbin](https://github.com/szabodanika/microbin) - 灵感来源
- [cloudflare-drop](https://github.com/oustn/cloudflare-drop) - 参考项目

---

**Elizabeth** - 让文件分享变得简单而强大

[开始使用](#快速开始) • [API 文档](#api-文档) • [贡献指南](#贡献指南)
