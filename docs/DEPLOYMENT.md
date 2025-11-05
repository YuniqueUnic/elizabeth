# Elizabeth 部署指南

本文档介绍如何使用 Docker 和 Docker Compose 部署 Elizabeth 文件分享与协作平台。

## 目录

- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [生产环境部署](#生产环境部署)
- [维护与管理](#维护与管理)
- [故障排查](#故障排查)

## 快速开始

### 前置要求

- Docker 20.10+
- Docker Compose 2.0+
- 至少 2GB 可用内存
- 至少 5GB 可用磁盘空间

### 一键部署

1. **克隆项目**

```bash
git clone <repository-url>
cd elizabeth
```

2. **配置环境变量**

```bash
# 复制环境变量模板
cp .env.docker .env

# 编辑 .env 文件，至少修改以下配置：
# - JWT_SECRET: 修改为至少 32 字符的随机字符串
# - NEXT_PUBLIC_API_URL: 建议保持为 /api/v1，确保浏览器请求通过前端代理
# - INTERNAL_API_URL: 配置 Next.js 服务器访问后端的内部地址（Docker 默认 http://elizabeth-backend:4092/api/v1）
# - NEXT_PUBLIC_APP_URL: 如果部署到生产环境，修改为实际的前端 URL
```

3. **启动服务**

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 查看服务状态
docker-compose ps
```

4. **访问应用**

- 前端：http://localhost:4001
- 后端 API: http://localhost:4092/api/v1
- API 文档：http://localhost:4092/api/v1/scalar

## 配置说明

### 环境变量

所有配置都通过环境变量进行管理。主要配置项包括：

#### 端口配置

```bash
BACKEND_PORT=4092          # 后端服务端口
FRONTEND_PORT=4001         # 前端服务端口
```

#### 前端配置

```bash
# 浏览器可见的 API 前缀 - 保持相对路径
NEXT_PUBLIC_API_URL=/api/v1

# Next.js 服务器访问后端的内部地址
INTERNAL_API_URL=http://localhost:4092/api/v1

# App URL - 前端公开访问地址
NEXT_PUBLIC_APP_URL=http://localhost:4001
```

#### 数据库配置

```bash
DB_MAX_CONNECTIONS=20      # 最大数据库连接数
DB_MIN_CONNECTIONS=5       # 最小数据库连接数
```

#### JWT 配置

```bash
# JWT 密钥 - 生产环境必须修改！
JWT_SECRET=your-secret-key-at-least-32-characters-long

JWT_TTL_SECONDS=7200                    # Token 有效期（2小时）
JWT_REFRESH_TTL_SECONDS=604800          # Refresh Token 有效期（7天）
JWT_MAX_REFRESH_COUNT=10                # 最大刷新次数
JWT_ENABLE_REFRESH_TOKEN_ROTATION=true  # 启用 Token 轮换
```

#### 房间配置

```bash
ROOM_MAX_SIZE=52428800         # 房间最大容量（50MB）
ROOM_MAX_TIMES_ENTERED=100     # 房间最大进入次数
```

#### 日志配置

```bash
LOG_LEVEL=info  # 日志级别: off, error, warn, info, debug, trace
```

#### 中间件配置

详细的中间件配置请参考 `.env.docker` 文件中的注释。

### 数据持久化

Docker Compose 使用命名卷来持久化数据：

- `backend-data`: 存储 SQLite 数据库文件
- `backend-storage`: 存储上传的文件

数据位置：

```bash
# 查看卷信息
docker volume inspect elizabeth_backend-data
docker volume inspect elizabeth_backend-storage

# 备份数据
docker run --rm -v elizabeth_backend-data:/data -v $(pwd):/backup alpine tar czf /backup/backend-data-backup.tar.gz -C /data .
docker run --rm -v elizabeth_backend-storage:/data -v $(pwd):/backup alpine tar czf /backup/backend-storage-backup.tar.gz -C /data .

# 恢复数据
docker run --rm -v elizabeth_backend-data:/data -v $(pwd):/backup alpine tar xzf /backup/backend-data-backup.tar.gz -C /data
docker run --rm -v elizabeth_backend-storage:/data -v $(pwd):/backup alpine tar xzf /backup/backend-storage-backup.tar.gz -C /data
```

## 生产环境部署

### 安全配置

1. **修改 JWT 密钥**

```bash
# 生成安全的随机密钥
openssl rand -base64 48

# 在 .env 文件中设置
JWT_SECRET=<生成的密钥>
```

2. **配置 CORS**

```bash
# 限制允许的源
MIDDLEWARE_CORS_ALLOWED_ORIGINS=https://yourdomain.com,https://www.yourdomain.com

# 如果需要携带凭证
MIDDLEWARE_CORS_ALLOW_CREDENTIALS=true
```

3. **启用 HTTPS**

建议在前面使用反向代理（如 Nginx 或 Traefik）来处理 HTTPS：

```nginx
# Nginx 配置示例
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

4. **启用速率限制**

```bash
MIDDLEWARE_RATE_LIMIT_ENABLED=true
MIDDLEWARE_RATE_LIMIT_PER_SECOND=10
MIDDLEWARE_RATE_LIMIT_BURST_SIZE=20
```

### 性能优化

1. **调整数据库连接池**

```bash
# 根据服务器资源调整
DB_MAX_CONNECTIONS=50
DB_MIN_CONNECTIONS=10
```

2. **启用压缩**

```bash
MIDDLEWARE_COMPRESSION_ENABLED=true
MIDDLEWARE_COMPRESSION_MIN_CONTENT_LENGTH=1024
```

3. **调整房间大小限制**

```bash
# 根据实际需求调整（单位：字节）
ROOM_MAX_SIZE=104857600  # 100MB
```

## 维护与管理

### 常用命令

```bash
# 启动服务
docker-compose up -d

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看日志
docker-compose logs -f [service_name]

# 查看服务状态
docker-compose ps

# 进入容器
docker-compose exec backend sh
docker-compose exec frontend sh

# 更新服务
docker-compose pull
docker-compose up -d --build

# 清理未使用的资源
docker system prune -a
```

### 监控

```bash
# 查看资源使用情况
docker stats

# 查看容器健康状态
docker-compose ps
```

### 日志管理

```bash
# 查看实时日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f backend
docker-compose logs -f frontend

# 查看最近 100 行日志
docker-compose logs --tail=100

# 导出日志
docker-compose logs > elizabeth-logs.txt
```

## 故障排查

### 后端无法启动

1. 检查日志：

```bash
docker-compose logs backend
```

2. 检查数据库文件权限：

```bash
docker-compose exec backend ls -la /app/data
```

3. 检查环境变量：

```bash
docker-compose exec backend env | grep ELIZABETH
```

### 前端无法连接后端

1. 检查网络连接：

```bash
docker-compose exec frontend ping backend
```

2. 检查环境变量：

```bash
docker-compose exec frontend env | grep NEXT_PUBLIC
```

3. 确认后端服务正常：

```bash
curl http://localhost:4092/api/v1/health
```

### 数据库迁移失败

1. 检查迁移文件：

```bash
docker-compose exec backend ls -la /app/migrations
```

2. 手动运行迁移：

```bash
docker-compose exec backend /app/board run
```

### 容器健康检查失败

1. 查看健康检查日志：

```bash
docker inspect elizabeth-backend | jq '.[0].State.Health'
docker inspect elizabeth-frontend | jq '.[0].State.Health'
```

2. 手动执行健康检查命令：

```bash
docker-compose exec backend sqlite3 /app/data/app.db "SELECT 1;"
```

## 升级指南

### 升级到新版本

```bash
# 1. 备份数据
./scripts/backup.sh

# 2. 拉取最新代码
git pull

# 3. 重新构建镜像
docker-compose build --no-cache

# 4. 停止旧服务
docker-compose down

# 5. 启动新服务
docker-compose up -d

# 6. 检查服务状态
docker-compose ps
docker-compose logs -f
```

## 开发环境

如果需要在 Docker 中运行开发环境：

```bash
# 使用开发配置
docker-compose -f docker-compose.dev.yml up -d
```

## 支持

如有问题，请查看：

- [项目文档](../README.md)
- [API 文档](http://localhost:4092/api/v1/scalar)
- [GitHub Issues](https://github.com/your-repo/issues)
