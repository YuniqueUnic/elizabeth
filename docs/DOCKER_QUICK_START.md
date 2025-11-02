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
# 1. 初始化环境配置
just docker-init

# 2. 编辑 .env 文件，设置 JWT_SECRET
# 生成安全的密钥
openssl rand -base64 48

# 将生成的密钥设置到 .env 文件中的 JWT_SECRET

# 3. 一键部署
just docker-deploy

# 4. 查看服务状态
just docker-status

# 5. 查看日志
just docker-logs
```

### 方法二：使用脚本

```bash
# 1. 初始化配置
cp .env.docker .env

# 2. 编辑 .env 文件，设置 JWT_SECRET
openssl rand -base64 48
# 将生成的密钥设置到 .env 文件中

# 3. 运行部署脚本
./scripts/deploy.sh

# 4. 查看状态
docker-compose ps

# 5. 查看日志
docker-compose logs -f
```

### 方法三：使用 Docker Compose

```bash
# 1. 初始化配置
cp .env.docker .env

# 2. 编辑 .env 文件
vim .env

# 3. 构建并启动
docker-compose up -d --build

# 4. 查看状态
docker-compose ps
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

# 部署相关
just docker-deploy          # 一键部署
just docker-build           # 构建镜像
just docker-up              # 启动服务
just docker-down            # 停止服务
just docker-restart         # 重启服务

# 监控相关
just docker-status          # 查看状态
just docker-logs            # 查看所有日志
just docker-logs backend    # 查看后端日志
just docker-logs frontend   # 查看前端日志
just docker-stats           # 查看资源使用

# 维护相关
just docker-backup          # 备份数据
just docker-restore <name>  # 恢复数据
just docker-clean           # 清理资源

# 调试相关
just docker-shell-backend   # 进入后端容器
just docker-shell-frontend  # 进入前端容器
just docker-validate        # 验证配置

# 别名
just dd                     # = docker-deploy
just db                     # = docker-build
just du                     # = docker-up
just ds                     # = docker-status
just dl                     # = docker-logs
just dc                     # = docker-clean
```

### Docker Compose 命令

```bash
# 启动服务
docker-compose up -d

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f
docker-compose logs -f backend
docker-compose logs -f frontend

# 进入容器
docker-compose exec backend sh
docker-compose exec frontend sh

# 重新构建
docker-compose build --no-cache
docker-compose up -d --build
```

## 🔧 配置说明

### 必须修改的配置

在 `.env` 文件中，以下配置**必须**在生产环境中修改：

```bash
# JWT 密钥 - 必须修改！至少 32 字符
JWT_SECRET=your-secure-secret-key-here

# 如果部署到公网，修改这些 URL
NEXT_PUBLIC_API_URL=https://api.yourdomain.com/api/v1
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

## 💾 数据备份与恢复

### 备份数据

```bash
# 使用 Just
just docker-backup

# 或使用脚本
./scripts/backup.sh
```

备份文件将保存在 `./backups/` 目录下。

### 恢复数据

```bash
# 查看可用的备份
ls -la backups/

# 使用 Just 恢复
just docker-restore elizabeth_backup_20240101_120000

# 或使用脚本
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
just docker-logs

# 检查配置
just docker-validate

# 检查容器状态
docker-compose ps
docker inspect elizabeth-backend
docker inspect elizabeth-frontend
```

### 后端数据库问题

```bash
# 进入后端容器
just docker-shell-backend

# 检查数据库
ls -la /app/data/
sqlite3 /app/data/app.db "SELECT 1;"

# 查看迁移文件
ls -la /app/migrations/
```

### 前端无法连接后端

```bash
# 检查网络连接
docker-compose exec frontend ping backend

# 检查环境变量
docker-compose exec frontend env | grep NEXT_PUBLIC

# 测试后端 API
curl http://localhost:4092/api/v1/health
```

### 重置所有数据

```bash
# 警告：这将删除所有数据！
just docker-clean

# 重新部署
just docker-deploy
```

## 🔄 更新应用

```bash
# 1. 备份当前数据
just docker-backup

# 2. 拉取最新代码
git pull

# 3. 重新构建并部署
just docker-build
just docker-down
just docker-up

# 或者一键更新
just docker-deploy
```

## 📊 监控

### 查看资源使用

```bash
# 使用 Just
just docker-stats

# 或使用 Docker 命令
docker stats
```

### 查看健康状态

```bash
# 查看服务状态
just docker-status

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

1. 查看日志：`just docker-logs`
2. 检查配置：`just docker-validate`
3. 查看状态：`just docker-status`
4. 参考[完整部署文档](./DEPLOYMENT.md)
5. 提交 Issue 到 GitHub
