# Docker 部署实现文档

> 本文档记录 Elizabeth 项目 Docker 部署功能的实现过程和技术细节。
>
> 创建时间：2025-11-02 最后更新：2025-11-03

## 概述

为 Elizabeth 项目实现了完整的 Docker 部署方案，包括：

- 多阶段 Docker 镜像构建
- Docker Compose 编排
- 环境变量配置管理
- 数据持久化
- 健康检查
- 自动化部署脚本
- Just/Make 任务集成

### 2025-11-03 更新摘要

- 为解决 macOS 上“Device busy or not
  ready”报错，将后端数据、存储与配置改为绑定宿主机 `docker/backend/*`
  目录，并新增 `scripts/docker_prepare_volumes.sh`
  进行端口检测与目录准备；同时增加 `app.database.journal_mode` 配置并在 Docker
  运行时改为 `DELETE`，避免 SQLite WAL 在绑定挂载上导致锁冲突。
- 简化 `justfile` 与 `Makefile` 的 Docker 任务，仅保留
  cache、二进制、镜像构建以及前后端容器的启动/停止/重建命令，保证 KISS 原则。
- 更新 `docs/DOCKER_QUICK_START.md`，同步新的命令、挂载目录结构以及 gRPC FUSE
  虚拟化切换建议（参考 HashCorp 支持文档）。
- 清理旧的命名卷说明与一键部署脚本依赖，强调显式构建/启动流程以便排查问题。

## 实现的功能

### 1. Docker 镜像

#### 后端镜像 (Dockerfile.backend)

**特性：**

- 多阶段构建，优化镜像大小
- 基础镜像：`rust:1.90-slim-bookworm` (builder), `debian:bookworm-slim`
  (runtime)
- 非 root 用户运行 (elizabeth:1000)
- 健康检查：SQLite 数据库连接测试
- 暴露端口：4092

**构建优化：**

- 分离依赖构建和应用构建
- 使用 cargo-chef 优化依赖缓存
- 最小化运行时依赖

#### 前端镜像 (Dockerfile.frontend)

**特性：**

- 三阶段构建：deps, builder, runner
- 基础镜像：`node:25-alpine`
- Next.js standalone 输出模式
- 非 root 用户运行 (nextjs:1001)
- 暴露端口：4001

**构建优化：**

- 分离依赖安装和应用构建
- 使用 standalone 模式减小镜像大小
- 仅复制必要的运行时文件

### 2. Docker Compose 编排

**服务定义：**

- `backend`: Rust 后端服务
- `frontend`: Next.js 前端服务

**网络配置：**

- 自定义桥接网络：`elizabeth-network`
- 服务间通过服务名通信

**数据持久化：**

- 通过 bind mount 挂载 `docker/backend/data`（SQLite 数据）与
  `docker/backend/storage/rooms`
- 配置文件 `docker/backend/config/backend.yaml` 随宿主机同步，便于外部编辑

**健康检查：**

- Backend: SQLite 连接测试
- Frontend: HTTP 健康检查
- 依赖管理：Frontend 依赖 Backend 健康

### 3. 配置管理

#### 环境变量系统

**配置层级：**

1. 默认配置 (代码中)
2. 配置文件 (`~/.config/elizabeth/config.yaml`)
3. 环境变量 (`ELIZABETH__APP__*`)

**配置分类：**

- 服务器配置 (地址、端口)
- 数据库配置 (URL、连接池)
- 存储配置 (根目录)
- JWT 配置 (密钥、过期时间)
- 房间配置 (大小限制、进入次数)
- 上传配置 (预留时间)
- 日志配置 (级别)
- 中间件配置 (CORS、限流、安全等)

**配置文件：**

- `.env.docker`: 环境变量模板
- `web/.env.example`: 前端环境变量示例

### 4. 部署脚本

#### deploy.sh - 自动化部署

**功能：**

- 验证 .env 文件存在
- 检查 JWT_SECRET 是否修改
- 验证 Docker 和 Docker Compose 安装
- 自动备份现有数据
- 构建镜像 (--no-cache)
- 启动服务
- 等待健康检查 (最多 30 次重试)
- 显示服务访问地址

#### backup.sh - 数据备份

**功能：**

- 创建时间戳备份目录
- 备份数据库卷
- 备份存储卷
- 生成备份信息文件
- 自动清理 7 天前的备份

#### restore.sh - 数据恢复

**功能：**

- 验证备份文件存在
- 确认恢复操作
- 停止服务
- 恢复数据
- 重启服务

### 5. Just/Make 集成

#### Just 任务 (justfile)

**构建相关：**

- `docker-backend-cache` (`dbc`): 构建后端依赖缓存 (planner)
- `docker-frontend-cache` (`dfc`): 构建前端依赖缓存 (deps)
- `docker-backend-binary` (`dbb`): 生成后端 builder 镜像
- `docker-frontend-binary` (`dfb`): 生成前端 builder 镜像
- `docker-backend-image` (`dbi`): 构建后端运行时镜像
- `docker-frontend-image` (`dfi`): 构建前端运行时镜像

**容器生命周期：**

- `docker-backend-up` (`dbu`): 准备挂载目录后启动后端容器
- `docker-frontend-up` (`dfu`): 启动前端容器（依赖后端就绪）
- `docker-backend-stop` (`dbs`): 停止后端容器
- `docker-frontend-stop` (`dfs`): 停止前端容器
- `docker-backend-recreate` (`dbr`): 强制重建后端容器
- `docker-frontend-recreate` (`dfr`): 强制重建前端容器

#### Makefile (备选方案)

Makefile 仅保留与 Just 相同的核心命令，方便未安装 Just 的环境调用。

### 6. 文档

#### DOCKER_QUICK_START.md

**内容：**

- 快速开始指南
- 三种部署方法 (Just/Scripts/Docker Compose)
- 常用命令参考
- 配置指南
- 备份恢复流程
- 故障排查
- 生产环境部署 (Nginx 示例)

#### DEPLOYMENT.md

**内容：**

- 完整部署指南
- 环境变量详细说明
- 数据持久化策略
- 生产环境安全配置
- 性能优化建议
- 维护命令
- 故障排查场景
- 升级流程

#### README.md 更新

**新增内容：**

- Docker 部署章节
- 技术栈说明 (前后端分离)
- 项目结构更新
- API 文档完善
- 配置管理说明
- 常用命令参考

## 技术细节

### 多阶段构建优化

**后端：**

```dockerfile
# Stage 1: Builder
FROM rust:1.90-slim-bookworm AS builder
# 构建应用

# Stage 2: Runtime
FROM debian:bookworm-slim
# 仅复制二进制文件和必要依赖
```

**前端：**

```dockerfile
# Stage 1: Dependencies
FROM node:25-alpine AS deps
# 安装依赖

# Stage 2: Builder
FROM node:25-alpine AS builder
# 构建应用

# Stage 3: Runner
FROM node:25-alpine AS runner
# 运行应用 (standalone 模式)
```

### 健康检查实现

**后端：**

```yaml
healthcheck:
  test: ["CMD", "sqlite3", "/app/data/app.db", "SELECT 1;"]
  interval: 10s
  timeout: 5s
  retries: 5
```

**前端：**

```yaml
healthcheck:
  test: [
    "CMD",
    "wget",
    "--quiet",
    "--tries=1",
    "--spider",
    "http://localhost:4001",
  ]
  interval: 10s
  timeout: 5s
  retries: 5
```

### 数据持久化

**宿主机目录：**

- `docker/backend/data` → `/app/data`（SQLite 数据库）
- `docker/backend/storage` → `/app/storage`（业务文件 / 房间内容）
- `docker/backend/config/backend.yaml` →
  `/app/config/backend.yaml`（运行时配置）

**实现细节：**

- Compose 使用 bind mount，并开启 `bind.create_host_path`
  以在缺失时自动创建目录。
- 新增
  `scripts/docker_prepare_volumes.sh`：在启动容器前准备目录并检测端口占用，避免
  SQLite 文件被宿主机锁定导致“Device busy or not ready”。
- 数据与配置直接落在仓库内，方便版本管理与手动备份。
- 后端配置新增 `app.database.journal_mode` 字段，默认仍为 `WAL`，而 Docker
  专用配置改为 `DELETE`，解决 VirtioFS/gRPC FUSE 上的 WAL 兼容性问题。

### 环境变量映射

**后端配置映射：**

```
ELIZABETH__APP__SERVER__ADDR=0.0.0.0
ELIZABETH__APP__SERVER__PORT=4092
ELIZABETH__APP__DATABASE__URL=sqlite:/app/data/app.db
ELIZABETH__APP__JWT__SECRET=${JWT_SECRET}
...
```

**前端配置：**

```
NEXT_PUBLIC_API_URL=http://localhost:4092/api/v1
NEXT_PUBLIC_APP_URL=http://localhost:4001
```

## 安全考虑

1. **非 root 用户**: 所有容器使用非 root 用户运行
2. **JWT 密钥**: 强制用户修改默认密钥
3. **CORS 配置**: 生产环境需配置允许的源
4. **安全头**: 默认启用安全相关 HTTP 头
5. **限流**: 可选的请求限流保护

## 性能优化

1. **镜像大小**: 使用 alpine/slim 基础镜像
2. **构建缓存**: 分离依赖和应用构建
3. **Standalone 模式**: Next.js 优化输出
4. **连接池**: 数据库连接池配置
5. **压缩**: 启用 HTTP 响应压缩

## 已知问题和限制

1. **SQLite 限制**: 不适合高并发场景，生产环境建议使用 PostgreSQL
2. **单机部署**: 当前方案为单机部署，不支持水平扩展
3. **文件存储**: 使用本地存储，未来可考虑对象存储
4. **macOS VirtioFS**: Docker Desktop 4.34+ 在启用 VirtioFS 时可能出现 bind
   mount“Device busy or not ready”，可切换到 gRPC FUSE 作为 workaround（参考
   HashCorp
   支持文档：<https://support.hashicorp.com/hc/en-us/articles/41463725654291-Nomad-on-macOS-Docker-Driver-Not-Detected-and-Nomad-Job-Fails-Due-to-Mount-Permission-Error>）

## 后续改进计划

1. **数据库支持**: 添加 PostgreSQL 支持
2. **对象存储**: 集成 S3 兼容存储
3. **监控**: 添加 Prometheus/Grafana 监控
4. **日志**: 集成 ELK 日志收集
5. **CI/CD**: GitHub Actions 自动构建镜像
6. **Kubernetes**: 提供 K8s 部署方案

## 参考资料

- [Docker 最佳实践](https://docs.docker.com/develop/dev-best-practices/)
- [Next.js Docker 部署](https://nextjs.org/docs/deployment#docker-image)
- [Rust Docker 优化](https://docs.docker.com/language/rust/)
