# Elizabeth 完整部署文档

本文档提供 Elizabeth 平台的生产环境部署指南，包括
Docker、裸金属服务器、云平台部署方案。

## 目录

- [部署概览](#部署概览)
- [架构设计](#架构设计)
- [Docker 部署](#docker-部署)
- [生产环境配置](#生产环境配置)
- [HTTPS 与反向代理](#https-与反向代理)
- [数据库管理](#数据库管理)
- [备份与恢复](#备份与恢复)
- [监控与日志](#监控与日志)
- [性能优化](#性能优化)
- [安全加固](#安全加固)
- [云平台部署](#云平台部署)
- [故障恢复](#故障恢复)

---

## 部署概览

### 部署选项

| 部署方式               | 适用场景           | 难度     | 成本  |
| ---------------------- | ------------------ | -------- | ----- |
| Docker Compose         | 小型团队、快速部署 | ⭐       | 低    |
| Kubernetes             | 大规模、高可用     | ⭐⭐⭐⭐ | 中-高 |
| 云平台 (AWS/Azure/GCP) | 托管服务、弹性扩展 | ⭐⭐⭐   | 中-高 |
| 裸金属服务器           | 完全控制、高性能   | ⭐⭐⭐   | 低-中 |

### 组件清单

```
┌─────────────────────────────────────────────────────────┐
│                  Nginx Gateway (443/80)                 │
│  • TLS 终止                                             │
│  • 静态资源缓存                                          │
│  • 反向代理                                              │
└──────────────┬──────────────────────────────────────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
┌──────────────┐  ┌──────────────┐
│   Frontend   │  │   Backend    │
│  (Next.js)   │  │   (Rust)     │
│   Port 4001  │  │   Port 4092  │
└──────────────┘  └──────┬───────┘
                         │
                         ▼
                  ┌──────────────┐
                  │  Database    │
                  │  (SQLite)    │
                  └──────────────┘
```

---

## 架构设计

### 单节点架构（推荐起步）

适用于：日活 < 1000，并发 < 100

```
服务器配置：
- CPU: 2-4 核心
- RAM: 4-8 GB
- 磁盘: 50-100 GB SSD
- 网络: 100 Mbps+

组件部署：
- Nginx: 反向代理 + TLS
- Backend: 1 实例
- Frontend: 1 实例
- Database: SQLite
```

### 高可用架构（生产推荐）

适用于：日活 > 1000，并发 > 100

```
                    ┌──────────────┐
                    │ Load Balancer│
                    │  (HAProxy)   │
                    └───────┬──────┘
                            │
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
    ┌──────────────┐┌──────────────┐┌──────────────┐
    │   Gateway 1  ││   Gateway 2  ││   Gateway 3  │
    └──────┬───────┘└──────┬───────┘└──────┬───────┘
           │               │               │
    ┌──────┴───────────────┴───────────────┴──────┐
    │                                              │
    ▼                                              ▼
┌──────────────┐                           ┌──────────────┐
│  Backend 1-N │                           │ Frontend 1-N │
│  (Stateless) │                           │  (Stateless) │
└──────┬───────┘                           └──────────────┘
       │
       ▼
┌──────────────┐      ┌──────────────┐
│  PostgreSQL  │◄─────│   Replica    │
│   (Primary)  │      │  (Read-only) │
└──────────────┘      └──────────────┘
       │
       ▼
┌──────────────┐
│ Shared Storage│
│  (NFS/S3)    │
└──────────────┘
```

---

## Docker 部署

### 快速部署

详见 [DOCKER_QUICK_START.md](./DOCKER_QUICK_START.md)

### 生产环境 Docker Compose（推荐做法）

生产环境建议直接基于仓库自带的 `docker-compose.yml`，用 `.env` 管理配置；如需
PostgreSQL，叠加 `docker-compose.postgres.yml`。

```bash
cp .env.docker .env
${EDITOR:-nano} .env

docker compose up -d --build

# 启用 PostgreSQL（可选）
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
```

如果你需要对外暴露 80/443（TLS 终止），建议：

- 在宿主机或云上放置独立的反向代理（Caddy/Nginx/Traefik）负责 80/443，再转发到
  `http://127.0.0.1:4001`。
- 或者维护一个 compose override，让 gateway 挂载你的 TLS 配置与证书。

---

## 生产环境配置

生产环境建议只维护两处配置：

1. `.env`（Docker Compose 注入）

- **必须**：`JWT_SECRET`（长度 >= 32）
- 数据库：`DATABASE_URL`（SQLite/PostgreSQL）
- 连接池：`DB_MAX_CONNECTIONS` / `DB_MIN_CONNECTIONS`
- 日志：`LOG_LEVEL` / `RUST_LOG`
- 房间/上传：`ROOM_MAX_SIZE` / `ROOM_MAX_TIMES_ENTERED` /
  `UPLOAD_RESERVATION_TTL_SECONDS`
- 中间件：`MIDDLEWARE_*`（详见 `.env.docker`）

2. `docker/backend/config/backend.yaml`（应用配置文件）

- `app.server.addr` / `app.server.port`
- `app.database.url`（Docker 内建议使用 `/app/data`）
- `app.database.journal_mode`（SQLite
  在不同宿主/文件系统下建议不同，详见该文件注释）
- `app.storage.root`

注意：

- YAML
  不会自动从环境变量插值；生产密钥等通过环境变量注入（实现：`crates/board/src/init/cfg_service.rs`）。
- 如需用环境变量覆盖任意配置字段，可使用 configrs
  前缀：`ELIZABETH__APP__...`（实现：`crates/configrs/src/lib.rs`）。

示例（SQLite 默认）：

```bash
cp .env.docker .env
${EDITOR:-nano} .env

docker compose up -d --build
```

示例（PostgreSQL）：

```bash
# 在 .env 中设置 DATABASE_URL 或 POSTGRES_PASSWORD

docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
```

## HTTPS 与反向代理

### 使用 Nginx + Let's Encrypt

#### 1. 安装 Certbot

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install certbot python3-certbot-nginx

# CentOS/RHEL
sudo yum install certbot python3-certbot-nginx
```

#### 2. 获取 SSL 证书

```bash
sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com
```

#### 3. Nginx 配置

`/etc/nginx/sites-available/elizabeth`:

```nginx
# HTTP → HTTPS 重定向
server {
    listen 80;
    listen [::]:80;
    server_name yourdomain.com www.yourdomain.com;

    # Let's Encrypt 验证
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }

    # 强制 HTTPS
    location / {
        return 301 https://$server_name$request_uri;
    }
}

# HTTPS 服务
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name yourdomain.com www.yourdomain.com;

    # SSL 证书
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
    ssl_trusted_certificate /etc/letsencrypt/live/yourdomain.com/chain.pem;

    # SSL 配置（Mozilla 中级配置）
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384';
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    ssl_stapling on;
    ssl_stapling_verify on;

    # 安全头
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # 文件上传大小限制（与后端一致）
    client_max_body_size 120m;
    client_body_buffer_size 128k;

    # 超时配置
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;

    # 日志
    access_log /var/log/nginx/elizabeth-access.log;
    error_log /var/log/nginx/elizabeth-error.log;

    # WebSocket
    location = /api/v1/ws {
        proxy_pass http://localhost:4001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }

    # Backend API
    location ^~ /api/v1/ {
        proxy_pass http://localhost:4001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Frontend（Next.js）
    location / {
        proxy_pass http://localhost:4001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Next.js 特定配置
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header X-Forwarded-Server $host;
    }

    # 静态资源缓存（Next.js）
    location ~* ^/_next/static/ {
        proxy_pass http://localhost:4001;
        proxy_cache_valid 200 60m;
        proxy_cache_bypass $http_cache_control;
        add_header Cache-Control "public, max-age=31536000, immutable";
    }
}
```

启用配置：

```bash
sudo ln -s /etc/nginx/sites-available/elizabeth /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

#### 4. 自动续期证书

```bash
# 测试续期
sudo certbot renew --dry-run

# Cron 任务（每天检查）
echo "0 0 * * * root certbot renew --quiet && systemctl reload nginx" | sudo tee -a /etc/crontab
```

---

### 使用 Caddy（自动 HTTPS）

`/etc/caddy/Caddyfile`:

```caddy
yourdomain.com {
    # 自动获取和续期 Let's Encrypt 证书

    # WebSocket
    @websocket {
        path /api/v1/ws
    }
    reverse_proxy @websocket localhost:4001 {
        transport http {
            versions h2c 1.1
        }
    }

    # API
    reverse_proxy /api/v1/* localhost:4001

    # Frontend
    reverse_proxy localhost:4001

    # 日志
    log {
        output file /var/log/caddy/elizabeth.log
        format json
    }

    # 安全头
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }
}
```

启动 Caddy：

```bash
sudo systemctl enable caddy
sudo systemctl start caddy
```

---

## 数据库管理

### SQLite 优化

#### 配置优化

```yaml
# backend.yaml
database:
  journal_mode: "wal" # Write-Ahead Logging，提升并发
  synchronous: "normal" # 平衡性能和安全
  cache_size: 10000 # 缓存大小（页数）
  mmap_size: 268435456 # 内存映射 256MB
  temp_store: "memory" # 临时表存储在内存
```

#### 维护任务

```bash
# 优化数据库
sqlite3 /var/lib/elizabeth/data/elizabeth.db "VACUUM;"

# 分析查询性能
sqlite3 /var/lib/elizabeth/data/elizabeth.db "ANALYZE;"

# 检查完整性
sqlite3 /var/lib/elizabeth/data/elizabeth.db "PRAGMA integrity_check;"
```

#### 定期维护脚本

`/usr/local/bin/elizabeth-db-maintenance.sh`:

```bash
#!/bin/bash
set -e

DB_PATH="/var/lib/elizabeth/data/elizabeth.db"
BACKUP_DIR="/var/backups/elizabeth"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# 备份
mkdir -p "$BACKUP_DIR"
sqlite3 "$DB_PATH" ".backup '$BACKUP_DIR/elizabeth_$TIMESTAMP.db'"

# VACUUM（整理碎片）
sqlite3 "$DB_PATH" "VACUUM;"

# ANALYZE（更新统计信息）
sqlite3 "$DB_PATH" "ANALYZE;"

# 检查完整性
sqlite3 "$DB_PATH" "PRAGMA integrity_check;" | tee -a /var/log/elizabeth/db-check.log

# 清理旧备份（保留7天）
find "$BACKUP_DIR" -name "elizabeth_*.db" -mtime +7 -delete

echo "Database maintenance completed at $(date)"
```

Cron 任务（每周执行）：

```bash
0 3 * * 0 /usr/local/bin/elizabeth-db-maintenance.sh >> /var/log/elizabeth/maintenance.log 2>&1
```

---

### 迁移到 PostgreSQL

当并发量增大或需要更强的一致性时，建议迁移到 PostgreSQL。

#### 1. 安装 PostgreSQL

```bash
# Ubuntu/Debian
sudo apt install postgresql postgresql-contrib

# 创建数据库和用户
sudo -u postgres psql
CREATE DATABASE elizabeth;
CREATE USER elizabeth WITH PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE elizabeth TO elizabeth;
\q
```

#### 2. 导出 SQLite 数据

```bash
# 安装转换工具
pip install pgloader

# 转换
pgloader sqlite:///var/lib/elizabeth/data/elizabeth.db \
  postgresql://elizabeth:secure_password@localhost/elizabeth
```

#### 3. 更新配置

```yaml
# backend.yaml
database:
  url: "postgresql://elizabeth:secure_password@localhost:5432/elizabeth"
  max_connections: 100
  min_connections: 10
  connection_timeout: 30
```

#### 4. PostgreSQL 性能优化

`/etc/postgresql/14/main/postgresql.conf`:

```ini
# 内存配置（假设 8GB RAM）
shared_buffers = 2GB
effective_cache_size = 6GB
maintenance_work_mem = 512MB
work_mem = 32MB

# WAL 配置
wal_buffers = 16MB
checkpoint_completion_target = 0.9
max_wal_size = 2GB
min_wal_size = 1GB

# 查询优化
random_page_cost = 1.1  # SSD
effective_io_concurrency = 200

# 连接
max_connections = 200
```

---

## 备份与恢复

### 自动备份策略

#### 完整备份脚本

`/usr/local/bin/elizabeth-backup.sh`:

```bash
#!/bin/bash
set -e

# 配置
BACKUP_ROOT="/var/backups/elizabeth"
DATA_DIR="/var/lib/elizabeth/data"
STORAGE_DIR="/var/lib/elizabeth/storage"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="$BACKUP_ROOT/$TIMESTAMP"

# 创建备份目录
mkdir -p "$BACKUP_DIR"

echo "[$(date)] Starting backup..."

# 1. 备份数据库
echo "Backing up database..."
if [ -f "$DATA_DIR/elizabeth.db" ]; then
    sqlite3 "$DATA_DIR/elizabeth.db" ".backup '$BACKUP_DIR/elizabeth.db'"
else
    # PostgreSQL
    pg_dump -U elizabeth elizabeth | gzip > "$BACKUP_DIR/elizabeth.sql.gz"
fi

# 2. 备份文件存储
echo "Backing up storage..."
tar -czf "$BACKUP_DIR/storage.tar.gz" -C "$STORAGE_DIR" .

# 3. 备份配置
echo "Backing up configuration..."
tar -czf "$BACKUP_DIR/config.tar.gz" /etc/elizabeth

# 4. 生成备份清单
cat > "$BACKUP_DIR/manifest.txt" <<EOF
Backup Date: $(date)
Hostname: $(hostname)
Database Size: $(du -sh $BACKUP_DIR/elizabeth.db* | cut -f1)
Storage Size: $(du -sh $BACKUP_DIR/storage.tar.gz | cut -f1)
Config Size: $(du -sh $BACKUP_DIR/config.tar.gz | cut -f1)
EOF

# 5. 清理旧备份
echo "Cleaning old backups..."
find "$BACKUP_ROOT" -maxdepth 1 -type d -mtime +$RETENTION_DAYS -exec rm -rf {} \;

# 6. 上传到远程存储（可选）
# aws s3 sync "$BACKUP_DIR" s3://your-bucket/elizabeth-backups/$TIMESTAMP/
# rclone sync "$BACKUP_DIR" remote:elizabeth-backups/$TIMESTAMP/

echo "[$(date)] Backup completed: $BACKUP_DIR"
du -sh "$BACKUP_DIR"
```

#### Cron 配置

```bash
# 每天凌晨 2 点备份
0 2 * * * /usr/local/bin/elizabeth-backup.sh >> /var/log/elizabeth/backup.log 2>&1

# 每周日凌晨 1 点完整备份到远程
0 1 * * 0 /usr/local/bin/elizabeth-backup.sh --remote >> /var/log/elizabeth/backup.log 2>&1
```

---

### 恢复流程

#### 从备份恢复

```bash
#!/bin/bash
set -e

BACKUP_DIR="/var/backups/elizabeth/20260120_020000"
DATA_DIR="/var/lib/elizabeth/data"
STORAGE_DIR="/var/lib/elizabeth/storage"

echo "⚠️  WARNING: This will overwrite current data!"
read -p "Continue? (yes/no): " confirm
[ "$confirm" != "yes" ] && exit 1

# 1. 停止服务
echo "Stopping services..."
docker compose down
# 或 systemctl stop elizabeth-backend elizabeth-frontend

# 2. 恢复数据库
echo "Restoring database..."
cp "$BACKUP_DIR/elizabeth.db" "$DATA_DIR/elizabeth.db"
# 或 PostgreSQL
# gunzip -c "$BACKUP_DIR/elizabeth.sql.gz" | psql -U elizabeth elizabeth

# 3. 恢复文件存储
echo "Restoring storage..."
rm -rf "$STORAGE_DIR"/*
tar -xzf "$BACKUP_DIR/storage.tar.gz" -C "$STORAGE_DIR"

# 4. 恢复配置
echo "Restoring configuration..."
tar -xzf "$BACKUP_DIR/config.tar.gz" -C /

# 5. 修复权限
chown -R elizabeth:elizabeth "$DATA_DIR" "$STORAGE_DIR"

# 6. 启动服务
echo "Starting services..."
docker compose up -d
# 或 systemctl start elizabeth-backend elizabeth-frontend

echo "✅ Restore completed!"
```

---

## 监控与日志

### Prometheus + Grafana 监控

#### 1. 暴露 Metrics 端点

在 `backend` 中集成 Prometheus metrics（需要添加依赖）：

```rust
// crates/board/src/metrics.rs
use prometheus::{Encoder, Registry, Counter, Histogram};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS: Counter = Counter::new(
        "http_requests_total", "Total HTTP requests"
    ).unwrap();

    pub static ref ROOM_CREATED: Counter = Counter::new(
        "rooms_created_total", "Total rooms created"
    ).unwrap();

    pub static ref FILE_UPLOADED: Counter = Counter::new(
        "files_uploaded_total", "Total files uploaded"
    ).unwrap();
}

pub fn metrics_handler() -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```

#### 2. Prometheus 配置

`/etc/prometheus/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: "elizabeth-backend"
    static_configs:
      - targets: ["localhost:4092"]
    metrics_path: "/metrics"

  - job_name: "node-exporter"
    static_configs:
      - targets: ["localhost:9100"]
```

#### 3. Grafana Dashboard

导入预配置的 Dashboard JSON 或创建自定义面板：

- 请求速率
- 错误率
- 响应时间（P50, P95, P99）
- 房间数量
- 文件上传量
- 数据库连接池状态
- 系统资源（CPU、内存、磁盘）

---

### 日志管理

#### 结构化日志配置

```yaml
# backend.yaml
logging:
  level: "info"
  format: "json" # 便于日志收集
  output: "file"
  file:
    path: "/var/log/elizabeth/backend.log"
    max_size: 100 # MB
    max_age: 30 # days
    max_backups: 10
    compress: true
```

#### 使用 ELK Stack

**Filebeat 配置** (`/etc/filebeat/filebeat.yml`):

```yaml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/elizabeth/*.log
    fields:
      app: elizabeth
      env: production
    json:
      keys_under_root: true
      add_error_key: true

output.elasticsearch:
  hosts: ["localhost:9200"]
  index: "elizabeth-%{+yyyy.MM.dd}"

setup.kibana:
  host: "localhost:5601"
```

---

## 性能优化

### 应用层优化

#### 1. 数据库连接池

```yaml
database:
  max_connections: 100 # 根据并发调整
  min_connections: 10
  connection_timeout: 30
  idle_timeout: 600
```

#### 2. 缓存策略

使用 Redis 缓存热点数据：

```yaml
cache:
  enabled: true
  backend: "redis"
  redis:
    url: "redis://localhost:6379/0"
    pool_size: 10
  ttl:
    room_info: 300 # 5 分钟
    token_validation: 60 # 1 分钟
```

#### 3. 文件存储优化

```yaml
storage:
  # 启用对象存储（大规模部署）
  backend: "s3"
  s3:
    region: "us-east-1"
    bucket: "elizabeth-files"
    endpoint: "https://s3.amazonaws.com"
  # 启用文件压缩
  compression:
    enabled: true
    algorithm: "zstd"
    level: 3
```

### 系统层优化

#### Linux 内核参数

`/etc/sysctl.conf`:

```ini
# 网络优化
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 4096
net.ipv4.ip_local_port_range = 10000 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15

# 文件描述符
fs.file-max = 2097152
```

应用：

```bash
sudo sysctl -p
```

#### 资源限制

`/etc/security/limits.conf`:

```
elizabeth soft nofile 65536
elizabeth hard nofile 65536
elizabeth soft nproc 32768
elizabeth hard nproc 32768
```

---

## 安全加固

### 1. 防火墙配置

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable

# 限制 SSH 访问
sudo ufw limit ssh
```

### 2. Fail2Ban 防护

`/etc/fail2ban/jail.local`:

```ini
[elizabeth-api]
enabled = true
port = http,https
filter = elizabeth-api
logpath = /var/log/nginx/elizabeth-access.log
maxretry = 10
findtime = 600
bantime = 3600
```

`/etc/fail2ban/filter.d/elizabeth-api.conf`:

```ini
[Definition]
failregex = ^<HOST> .* "POST /api/v1/rooms/.*/tokens.*" 401
            ^<HOST> .* "POST /api/v1/rooms/.*/tokens.*" 403
ignoreregex =
```

### 3. SELinux/AppArmor

Ubuntu AppArmor 配置示例：

`/etc/apparmor.d/elizabeth-backend`:

```
#include <tunables/global>

/app/board {
  #include <abstractions/base>

  # 允许网络
  network inet stream,
  network inet6 stream,

  # 允许读取配置
  /app/config/** r,

  # 允许读写数据
  /app/data/** rw,
  /app/storage/** rw,

  # 禁止执行其他程序
  deny /bin/** x,
  deny /usr/bin/** x,
}
```

---

## 云平台部署

### AWS 部署

#### 使用 ECS (Elastic Container Service)

**架构图：**

```
Internet → ALB → ECS Tasks (Backend + Frontend)
                    ↓
                  RDS (PostgreSQL)
                    ↓
                  S3 (File Storage)
```

**步骤：**

1. **创建 ECR 仓库并推送镜像**

```bash
# 登录 ECR
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com

# 构建并推送
docker build -t elizabeth-backend -f Dockerfile.backend .
docker tag elizabeth-backend:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/elizabeth-backend:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/elizabeth-backend:latest

# 同样处理 frontend
```

2. **创建 RDS 实例**

```bash
aws rds create-db-instance \
  --db-instance-identifier elizabeth-db \
  --db-instance-class db.t3.small \
  --engine postgres \
  --master-username elizabeth \
  --master-user-password <password> \
  --allocated-storage 20 \
  --vpc-security-group-ids <sg-id> \
  --db-subnet-group-name <subnet-group>
```

3. **创建 ECS 任务定义**

`task-definition.json`:

```json
{
  "family": "elizabeth",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [
    {
      "name": "backend",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/elizabeth-backend:latest",
      "portMappings": [{ "containerPort": 4092 }],
      "environment": [
        {
          "name": "DATABASE_URL",
          "value": "postgresql://elizabeth:<password>@<rds-endpoint>:5432/elizabeth"
        },
        { "name": "STORAGE_BACKEND", "value": "s3" },
        { "name": "S3_BUCKET", "value": "elizabeth-files" }
      ],
      "secrets": [
        { "name": "JWT_SECRET", "valueFrom": "arn:aws:secretsmanager:..." }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/elizabeth",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "backend"
        }
      }
    }
  ]
}
```

4. **创建 ALB 和目标组**

```bash
# 创建 ALB
aws elbv2 create-load-balancer \
  --name elizabeth-alb \
  --subnets <subnet-1> <subnet-2> \
  --security-groups <sg-id>

# 创建目标组
aws elbv2 create-target-group \
  --name elizabeth-backend \
  --protocol HTTP \
  --port 4092 \
  --vpc-id <vpc-id> \
  --target-type ip \
  --health-check-path /api/v1/health
```

---

### DigitalOcean App Platform

创建 `app.yaml`:

```yaml
name: elizabeth
region: nyc
services:
  - name: backend
    dockerfile_path: Dockerfile.backend
    github:
      repo: YuniqueUnic/elizabeth # 或你的 fork
      branch: main
    envs:
      - key: JWT_SECRET
        scope: RUN_TIME
        type: SECRET
        value: <your-secret>
      - key: DATABASE_URL
        scope: RUN_TIME
        value: ${db.DATABASE_URL}
    health_check:
      http_path: /api/v1/health
    http_port: 4092
    instance_count: 2
    instance_size_slug: basic-xs

  - name: frontend
    dockerfile_path: Dockerfile.frontend
    github:
      repo: YuniqueUnic/elizabeth # 或你的 fork
      branch: main
    envs:
      - key: NEXT_PUBLIC_API_URL
        value: /api/v1
      - key: INTERNAL_API_URL
        value: http://backend:4092/api/v1
    http_port: 4001
    instance_count: 1
    routes:
      - path: /

databases:
  - name: db
    engine: PG
    version: "14"
    size: db-s-1vcpu-1gb
```

部署：

```bash
doctl apps create --spec app.yaml
```

---

## 故障恢复

### 常见问题诊断

#### 1. 服务无响应

```bash
# 检查服务状态
docker compose ps
systemctl status elizabeth-backend

# 检查日志
docker compose logs backend --tail=100
journalctl -u elizabeth-backend -n 100

# 检查端口
netstat -tlnp | grep 4092
```

#### 2. 数据库锁定

```bash
# SQLite 锁定
# 检查是否有其他进程访问
fuser /var/lib/elizabeth/data/elizabeth.db

# 强制解锁（危险，仅紧急情况）
sqlite3 /var/lib/elizabeth/data/elizabeth.db ".timeout 5000"
```

#### 3. 磁盘空间不足

```bash
# 检查磁盘使用
df -h

# 清理 Docker 资源
docker system prune -a -f

# 清理日志
journalctl --vacuum-size=500M

# 清理旧备份
find /var/backups/elizabeth -mtime +7 -delete
```

---

**文档版本：** 1.0.0 **最后更新：** 2026-01-20 **适用版本：** Elizabeth v0.1.0+
