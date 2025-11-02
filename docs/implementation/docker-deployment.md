# Docker 部署实现文档

> 本文档记录 Elizabeth 项目 Docker 部署功能的实现过程和技术细节。
>
> 创建时间：2025-11-02 最后更新：2025-11-02

## 概述

为 Elizabeth 项目实现了完整的 Docker 部署方案，包括：

- 多阶段 Docker 镜像构建
- Docker Compose 编排
- 环境变量配置管理
- 数据持久化
- 健康检查
- 自动化部署脚本
- Just/Make 任务集成

## 实现的功能

### 1. Docker 镜像

#### 后端镜像 (Dockerfile.backend)

**特性：**

- 多阶段构建，优化镜像大小
- 基础镜像：`rust:1.83-slim-bookworm` (builder), `debian:bookworm-slim`
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
- 基础镜像：`node:20-alpine`
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

- `backend-data`: SQLite 数据库存储
- `backend-storage`: 文件存储

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

**部署任务：**

- `docker-deploy` (dd): 一键部署
- `docker-build` (db): 构建镜像
- `docker-up` (du): 启动服务
- `docker-down`: 停止服务
- `docker-restart`: 重启服务

**监控任务：**

- `docker-status` (ds): 查看状态
- `docker-logs` (dl): 查看日志
- `docker-stats`: 资源使用

**维护任务：**

- `docker-backup`: 备份数据
- `docker-restore`: 恢复数据
- `docker-clean` (dc): 清理资源

**调试任务：**

- `docker-shell-backend`: 进入后端容器
- `docker-shell-frontend`: 进入前端容器
- `docker-validate`: 验证配置

**工具任务：**

- `docker-init`: 初始化环境

#### Makefile (备选方案)

为不使用 Just 的用户提供相同功能的 Makefile。

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
FROM rust:1.83-slim-bookworm AS builder
# 构建应用

# Stage 2: Runtime
FROM debian:bookworm-slim
# 仅复制二进制文件和必要依赖
```

**前端：**

```dockerfile
# Stage 1: Dependencies
FROM node:20-alpine AS deps
# 安装依赖

# Stage 2: Builder
FROM node:20-alpine AS builder
# 构建应用

# Stage 3: Runner
FROM node:20-alpine AS runner
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

**卷定义：**

```yaml
volumes:
  backend-data:
    driver: local
  backend-storage:
    driver: local
```

**挂载点：**

- Backend: `/app/data` (数据库), `/app/storage` (文件)
- 使用命名卷确保数据持久化

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
