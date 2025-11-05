# Elizabeth Docker 快速开始指南

> 本文档提供 Elizabeth 项目的 Docker 部署快速开始指南。
>
> 完整的部署文档请参考 [DEPLOYMENT.md](./DEPLOYMENT.md)

本指南帮助您快速使用 Docker 部署 Elizabeth 文件分享与协作平台。

## 📋 前置要求

- Docker 20.10+
- Docker Compose 2.0+
- Just (可选，用于简化命令)

安装 Just:

```bash
# macOS
brew install just

# Linux
cargo install just

# 或者直接下载二进制文件
# https://github.com/casey/just
```

## 🚀 快速部署

### 方法一：使用 Just (推荐)

```bash
# 1. 构建镜像（首次部署或依赖更新时执行）
just docker-backend-cache
just docker-backend-image
just docker-frontend-cache
just docker-frontend-image

# 2. 启动容器（会自动检查端口占用并准备挂载目录）
just docker-backend-up
just docker-frontend-up

# 3. 查看状态 / 日志
docker compose ps
docker compose logs -f backend
docker compose logs -f frontend
```

### 方法二：使用脚本

```bash
# 1. 初始化配置
cp .env.docker .env

# 2. 编辑 .env 文件，设置 JWT_SECRET
openssl rand -base64 48
# 将生成的密钥设置到 .env 文件中

# 3. 准备挂载目录与检测端口
./scripts/docker_prepare_volumes.sh

# 4. 构建并启动
docker compose build backend frontend
docker compose up -d backend frontend

# 5. 查看状态 / 日志
docker compose ps
docker compose logs -f
```

### 方法三：纯 Docker Compose

```bash
# 1. 初始化配置
cp .env.docker .env

# 2. 编辑 .env 文件
vim .env

# 3. 准备挂载目录
./scripts/docker_prepare_volumes.sh

# 4. 构建并启动
docker compose up -d --build backend frontend

# 5. 查看状态
docker compose ps
```

## 🎯 访问应用

部署成功后，您可以访问：

- **前端界面**: http://localhost:4001
- **后端 API**: http://localhost:4092/api/v1
- **API 文档**: http://localhost:4092/api/v1/scalar

## 📝 常用命令

### Just 命令

```bash
# 查看所有可用命令
just --list

# 构建缓存 / 二进制 / 镜像
just docker-backend-cache   # 后端依赖缓存 (planner)
just docker-frontend-cache  # 前端依赖缓存 (deps)
just docker-backend-binary  # 后端 builder 镜像
just docker-frontend-binary # 前端 builder 镜像
just docker-backend-image   # 后端运行时镜像
just docker-frontend-image  # 前端运行时镜像

# 容器生命周期
just docker-backend-up      # 启动后端容器
just docker-frontend-up     # 启动前端容器
just docker-backend-stop    # 停止后端容器
just docker-frontend-stop   # 停止前端容器
just docker-backend-recreate # 强制重建后端容器
just docker-frontend-recreate # 强制重建前端容器

# 别名
just dbc  # = docker-backend-cache
just dfc  # = docker-frontend-cache
just dbb  # = docker-backend-binary
just dfb  # = docker-frontend-binary
just dbi  # = docker-backend-image
just dfi  # = docker-frontend-image
just dbu  # = docker-backend-up
just dfu  # = docker-frontend-up
just dbs  # = docker-backend-stop
just dfs  # = docker-frontend-stop
just dbr  # = docker-backend-recreate
just dfr  # = docker-frontend-recreate
```

### Docker Compose 命令

```bash
# 准备挂载目录
./scripts/docker_prepare_volumes.sh

# 启动服务
docker compose up -d backend frontend

# 重启服务
docker compose restart backend frontend

# 查看状态
docker compose ps

# 查看日志
docker compose logs -f
docker compose logs -f backend
docker compose logs -f frontend

# 进入容器
docker compose exec backend sh
docker compose exec frontend sh

# 重新构建
docker compose build --no-cache backend frontend
docker compose up -d --build backend frontend
```

## 🔧 配置说明

### 必须修改的配置

在 `.env` 文件中，以下配置**必须**在生产环境中修改：

```bash
# JWT 密钥 - 必须修改！至少 32 字符
JWT_SECRET=your-secure-secret-key-here

# 如果部署到公网，修改这些 URL
NEXT_PUBLIC_API_URL=/api/v1
INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1
NEXT_PUBLIC_APP_URL=https://yourdomain.com
```

### 可选配置

```bash
# 端口配置
BACKEND_PORT=4092
FRONTEND_PORT=4001

# 房间配置
ROOM_MAX_SIZE=52428800              # 50MB
ROOM_MAX_TIMES_ENTERED=100

# 日志级别
LOG_LEVEL=info                      # off, error, warn, info, debug, trace

# CORS 配置（生产环境建议限制）
MIDDLEWARE_CORS_ALLOWED_ORIGINS=*   # 生产环境改为具体域名
```

完整配置说明请参考 `.env.docker` 文件中的注释。

### Docker 数据挂载目录

仓库已经预置以下可写目录，便于通过宿主机直接管理数据与配置：

- `docker/backend/data`：持久化 SQLite 数据库文件
- `docker/backend/storage/rooms`：房间内容与上传文件存储目录
- `docker/backend/config/backend.yaml`：后端 Docker 运行时使用的配置文件
- `app.database.journal_mode`：默认改为 `delete`，避免 SQLite WAL 在 macOS
  VirtioFS/gRPC FUSE 上触发 `Device or resource busy`

`just docker-backend-up` 与 `scripts/docker_prepare_volumes.sh`
会自动创建缺失的目录，并在端口冲突时给出提示。若需要自定义配置，可直接编辑上述
YAML 文件后重建容器。

## 💾 数据备份与恢复

### 备份数据

```bash
# 使用脚本备份
./scripts/backup.sh
```

备份文件将保存在 `./backups/` 目录下。

### 恢复数据

```bash
# 查看可用的备份
ls -la backups/

# 通过脚本恢复
./scripts/restore.sh elizabeth_backup_20240101_120000
```

### 手动备份

```bash
# 备份数据库
docker run --rm \
  -v elizabeth_backend-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/manual_data_backup.tar.gz -C /data .

# 备份存储
docker run --rm \
  -v elizabeth_backend-storage:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/manual_storage_backup.tar.gz -C /data .
```

## 🔍 故障排查

### 服务无法启动

```bash
# 查看详细日志
docker compose logs -f backend
docker compose logs -f frontend

# 检查配置
docker compose config

# 检查容器状态
docker compose ps
docker inspect elizabeth-backend
docker inspect elizabeth-frontend
```

### 后端数据库问题

```bash
# 进入后端容器
docker compose exec backend sh

# 检查数据库
ls -la /app/data/
sqlite3 /app/data/app.db "SELECT 1;"

# 查看迁移文件
ls -la /app/migrations/
```

### 前端无法连接后端

```bash
# 检查网络连接
docker compose exec frontend ping backend

# 检查环境变量
docker compose exec frontend env | grep NEXT_PUBLIC

# 测试后端 API
curl http://localhost:4092/api/v1/health
```

### macOS 出现“Device busy or not ready”

1. 先运行 `./scripts/docker_prepare_volumes.sh`，脚本会检测端口 4092
   是否被本地进程占用。
2. 确认本地未同时运行 `cargo run -p elizabeth-board -- run` 等后端服务，以避免
   SQLite 文件被锁定。
3. 检查 `docker/backend/config/backend.yaml` 中 `app.database.journal_mode`
   是否设为 `delete`（Docker 默认配置已经调整为该值，若改成 `wal` 极易复现
   EBUSY）。修改后重启容器即可生效。
4. 若仍然失败，可在 Docker Desktop → Settings → General 中将 _Virtualization
   framework_ 切换为 **gRPC FUSE**，该方案已被 HashCorp 支持文档验证可缓解 macOS
   上的挂载权限错误
   [[来源](https://support.hashicorp.com/hc/en-us/articles/41463725654291-Nomad-on-macOS-Docker-Driver-Not-Detected-and-Nomad-Job-Fails-Due-to-Mount-Permission-Error)].

### 重置所有数据

```bash
# 警告：这将删除所有数据！
docker compose down -v
rm -rf docker/backend/data/*
rm -rf docker/backend/storage/rooms/*

# 重新部署
./scripts/docker_prepare_volumes.sh
docker compose up -d backend frontend
```

## 🔄 更新应用

```bash
# 1. 备份当前数据
./scripts/backup.sh

# 2. 拉取最新代码
git pull

# 3. 重新构建并部署
just docker-backend-image
just docker-frontend-image
just docker-backend-recreate
just docker-frontend-recreate

# 或者一键更新
docker compose up -d --build backend frontend
```

## 📊 监控

### 查看资源使用

```bash
docker stats
```

### 查看健康状态

```bash
# 查看服务状态
docker compose ps

# 查看健康检查详情
docker inspect elizabeth-backend | jq '.[0].State.Health'
docker inspect elizabeth-frontend | jq '.[0].State.Health'
```

## 🌐 生产环境部署

### 使用反向代理 (Nginx)

```nginx
server {
    listen 80;
    server_name yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # 前端
    location / {
        proxy_pass http://localhost:4001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # 后端 API
    location /api/ {
        proxy_pass http://localhost:4092;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 环境变量配置

```bash
# .env 文件
NEXT_PUBLIC_API_URL=https://yourdomain.com/api/v1
NEXT_PUBLIC_APP_URL=https://yourdomain.com
MIDDLEWARE_CORS_ALLOWED_ORIGINS=https://yourdomain.com
JWT_SECRET=<your-secure-secret-key>
```

## 📚 更多文档

- [完整部署文档](./DEPLOYMENT.md)
- [项目 README](../README.md)
- [前端文档](../web/README.md)

## 🆘 获取帮助

如遇问题，请：

1. 查看日志：`docker compose logs -f`
2. 检查配置：`docker compose config`
3. 查看状态：`docker compose ps`
4. 参考[完整部署文档](./DEPLOYMENT.md)
5. 提交 Issue 到 GitHub
