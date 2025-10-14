# Elizabeth

Elizabeth 是一个基于 Rust
的文件分享和协作平台项目，旨在提供简单、安全、高效的文件共享解决方案。

## 项目概述

Elizabeth
项目致力于构建一个现代化的文件分享系统，支持多种文件类型、实时协作和高级安全特性。项目采用模块化设计，使用
Rust 语言确保高性能和内存安全。

### 核心特性

- 🚀 **高性能**: 基于 Rust 的高性能文件处理
- 🔒 **安全性**: 内存安全和数据加密
- 📁 **多格式支持**: 文本、图片、代码片段等多种文件类型
- 🌐 **Web 界面**: 现代化的用户界面
- ☁️ **云存储**: 集成 Cloudflare R2 等 S3 兼容存储
- 🏠 **Room 系统**: 支持密码保护的房间系统，实现安全的内容分享

### Room CRUD 功能

项目已实现完整的 Room CRUD (Create, Read, Update, Delete) 功能：

- ✅ **创建房间**: 支持设置密码、过期时间、访问限制
- ✅ **查询房间**: 按名称或 ID 查询房间信息
- ✅ **更新房间**: 修改房间配置和权限设置
- ✅ **删除房间**: 安全删除房间及其相关内容
- ✅ **权限控制**: 支持编辑、下载、预览权限管理
- ✅ **过期管理**: 自动处理过期房间

## 项目结构

```
elizabeth/
├── crates/
│   └── board/           # 核心板块功能
│       ├── src/
│       │   ├── models/          # 数据模型
│       │   ├── repository/      # 数据访问层
│       │   ├── handlers/        # HTTP处理层
│       │   ├── route/           # 路由定义
│       │   ├── db/              # 数据库模块
│       │   └── tests/           # 测试模块
│       └── migrations/          # 数据库迁移文件
├── docs/
│   ├── research.md      # 研究和设计文档
│   ├── database-implementation.md  # 数据库实现文档
│   ├── room-crud-testing.md       # Room CRUD 测试报告
│   ├── room-crud-refactor.md      # Room CRUD 重构文档
│   ├── architecture.md            # 项目架构文档
│   ├── api-reference.md           # API 参考文档
│   ├── room-crud-implementation.md # Room CRUD 实现文档
│   ├── development-guide.md       # 开发指南
│   ├── release-plz.md   # 发布系统文档
│   ├── github-actions.md # CI/CD 文档
│   └── Tasks.md          # 项目任务跟踪
├── .github/
│   └── workflows/
│       └── release-plz.yml # 自动发布工作流
├── .release-plz.toml    # release-plz 配置
├── CHANGELOG.md         # 变更日志
├── Cargo.toml          # 项目配置
└── README.md           # 项目说明
```

## 技术栈

### 后端技术

- **Rust 1.90+**: 核心编程语言
- **Axum 0.8.6**: 异步 Web 框架
- **SQLx 0.8**: 异步 SQL 工具包，支持编译时查询检查
- **SQLite**: 轻量级数据库
- **Tokio**: 异步运行时
- **Serde**: 序列化/反序列化
- **Utoipa**: OpenAPI 文档生成

### 架构模式

- **Repository 模式**: 数据访问层抽象
- **分层架构**: 模型、仓库、处理器、路由清晰分离
- **依赖注入**: 使用 Axum State 管理依赖
- **错误处理**: 统一的错误处理机制

## 快速开始

### 环境要求

- Rust 1.90+
- Git
- SQLite 3

### 安装和构建

1. **克隆仓库**
   ```bash
   git clone https://github.com/your-username/elizabeth.git
   cd elizabeth
   ```

2. **构建项目**
   ```bash
   cargo build --release
   ```

3. **运行项目**
   ```bash
   cargo run
   ```

   服务将在 `http://127.0.0.1:8080` 启动

### 开发环境设置

1. **安装开发依赖**
   ```bash
   cargo install --dev release-plz
   cargo install --dev git-cliff
   cargo install --dev cargo-semver-checks
   ```

2. **运行测试**
   ```bash
   cargo test
   ```

3. **检查代码格式**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

## API 文档

### Room CRUD API

项目提供完整的 Room CRUD REST API，支持以下操作：

#### 创建房间

```http
POST /api/v1/rooms/{name}?password={password}
```

#### 查询房间

```http
GET /api/v1/rooms/{name}
```

#### 删除房间

```http
DELETE /api/v1/rooms/{name}
```

### OpenAPI 文档

启动服务后，可以通过以下地址访问 API 文档：

- Swagger UI: `http://127.0.0.1:8080/swagger-ui/`
- OpenAPI JSON: `http://127.0.0.1:8080/api-docs/openapi.json`

## 使用示例

### 创建房间

```bash
# 创建带密码的房间
curl -X POST "http://127.0.0.1:8080/api/v1/rooms/myroom?password=secret123"
```

### 查询房间

```bash
# 查询房间信息
curl -X GET "http://127.0.0.1:8080/api/v1/rooms/myroom"
```

### 删除房间

```bash
# 删除房间
curl -X DELETE "http://127.0.0.1:8080/api/v1/rooms/myroom"
```

## 数据库设计

### 房间表 (rooms)

| 字段                  | 类型     | 描述                               |
| --------------------- | -------- | ---------------------------------- |
| id                    | INTEGER  | 主键，自增                         |
| name                  | TEXT     | 房间名称，唯一                     |
| password              | TEXT     | 房间密码（可选）                   |
| status                | INTEGER  | 房间状态（0:开放，1:锁定，2:关闭） |
| max_size              | INTEGER  | 最大文件大小（字节）               |
| current_size          | INTEGER  | 当前文件大小（字节）               |
| max_times_entered     | INTEGER  | 最大进入次数                       |
| current_times_entered | INTEGER  | 当前进入次数                       |
| expire_at             | DATETIME | 过期时间（可选）                   |
| created_at            | DATETIME | 创建时间                           |
| updated_at            | DATETIME | 更新时间                           |
| allow_edit            | BOOLEAN  | 允许编辑                           |
| allow_download        | BOOLEAN  | 允许下载                           |
| allow_preview         | BOOLEAN  | 允许预览                           |

详细的数据库设计请参考
[`docs/database-implementation.md`](./docs/database-implementation.md)。

## 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试模块
cargo test room_repository_tests
cargo test api_integration_tests
```

### 测试覆盖

- ✅ Repository 单元测试（8/8 通过）
- ✅ 数据库操作测试
- ⚠️ API 集成测试（待修复）
- ✅ 手动 API 测试验证

详细的测试报告请参考
[`docs/room-crud-testing.md`](./docs/room-crud-testing.md)。

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
[`docs/release-plz.md`](./docs/release-plz.md)。

## 开发指南

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 遵循 Rust 官方编码规范
- 编写单元测试和集成测试

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

详细的开发指南请参考
[`docs/development-guide.md`](./docs/development-guide.md)。

## 项目架构

### 整体架构

项目采用分层架构模式，包含以下层次：

1. **路由层** (Route): 定义 API 端点和路由规则
2. **处理层** (Handler): 处理 HTTP 请求和响应
3. **仓库层** (Repository): 数据访问抽象
4. **模型层** (Model): 数据模型定义

### 模块说明

- **models**: 定义数据模型和 API 响应模型
- **repository**: 实现数据访问逻辑，使用 Repository 模式
- **handlers**: 处理 HTTP 请求，包含业务逻辑
- **route**: 定义 API 路由和中间件
- **db**: 数据库连接和配置管理

详细的架构说明请参考 [`docs/architecture.md`](./docs/architecture.md)。

## 文档

### 核心文档

- [`docs/research.md`](./docs/research.md) - 研究和设计文档
- [`docs/database-implementation.md`](./docs/database-implementation.md) -
  数据库实现详细文档
- [`docs/room-crud-implementation.md`](./docs/room-crud-implementation.md) -
  Room CRUD 功能实现文档
- [`docs/room-crud-testing.md`](./docs/room-crud-testing.md) - Room CRUD
  测试报告
- [`docs/room-crud-refactor.md`](./docs/room-crud-refactor.md) - Room CRUD
  重构文档
- [`docs/architecture.md`](./docs/architecture.md) - 项目架构文档
- [`docs/api-reference.md`](./docs/api-reference.md) - API 参考文档
- [`docs/development-guide.md`](./docs/development-guide.md) - 开发指南

### 工具文档

- [`docs/release-plz.md`](./docs/release-plz.md) - 发布系统详细文档
- [`docs/github-actions.md`](./docs/github-actions.md) - GitHub Actions 配置文档
- [`CHANGELOG.md`](./CHANGELOG.md) - 项目变更日志
- [`docs/Tasks.md`](./docs/Tasks.md) - 项目任务跟踪

## 贡献指南

我们欢迎所有形式的贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 贡献类型

- 🐛 Bug 修复
- ✨ 新功能开发
- 📝 文档改进
- 🎨 代码优化和重构
- ⚡ 性能优化
- 🧪 测试覆盖

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 联系方式

- 项目主页：https://github.com/your-username/elizabeth
- 问题反馈：https://github.com/your-username/elizabeth/issues
- 讨论区：https://github.com/your-username/elizabeth/discussions

## 致谢

感谢所有为 Elizabeth 项目做出贡献的开发者和社区成员！

### 主要依赖

- [release-plz](https://release-plz.ieni.dev/) - 自动化发布工具
- [git-cliff](https://github.com/orhun/git-cliff) - Changelog 生成工具
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) -
  语义化版本检查
- [Axum](https://github.com/tokio-rs/axum) - 异步 Web 框架
- [SQLx](https://github.com/launchbadge/sqlx) - 异步 SQL 工具包

### 相关项目

- [microbin](https://github.com/szabodanika/microbin) - 灵感来源之一
- [cloudflare-drop](https://github.com/oustn/cloudflare-drop) - 参考项目

---

**Elizabeth** - 让文件分享变得简单而强大 🚀

最后更新：2025-10-14
