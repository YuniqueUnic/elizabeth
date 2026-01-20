# Elizabeth Docker 快速开始

目标：用 `docker compose` 在一台机器上跑起来（Gateway + Frontend +
Backend），默认使用 SQLite；可选 PostgreSQL。

## TL;DR（默认 SQLite）

```bash
git clone https://github.com/YuniqueUnic/elizabeth.git
cd elizabeth

cp .env.docker .env
# 生产环境务必修改 JWT_SECRET（长度 >= 32）
${EDITOR:-nano} .env

docker compose up -d --build
docker compose ps
```

访问：

- 前端：`http://localhost:4001`
- OpenAPI：`http://localhost:4001/api/v1/docs`
- 健康检查：`http://localhost:4001/api/v1/health`

数据默认落盘在仓库目录（可通过 `.env` 覆盖）：

- `ELIZABETH_DATA_DIR`（SQLite DB 文件）
- `ELIZABETH_STORAGE_DIR`（上传文件）
- `ELIZABETH_BACKEND_CONFIG`（后端配置文件挂载路径）

## 可选：使用 PostgreSQL

后端已支持 SQLite / PostgreSQL：按 `DATABASE_URL`
自动选择驱动，并自动切换迁移目录（源码：SQLite →
`crates/board/migrations`，PostgreSQL → `crates/board/migrations_pg`；Docker
runtime：`/app/migrations` 与 `/app/migrations_pg`）。

### 方式 A（推荐）：使用本仓库提供的 compose override

1. 编辑 `.env`（至少设置密码）：

```bash
POSTGRES_PASSWORD=please-change-me
```

2. 启动（会额外启动一个 `postgres` 容器，并将后端 `DATABASE_URL` 默认指向它）：

```bash
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
```

> PostgreSQL 数据默认存储在 Docker volume `postgres-data`。

### 方式 B：接入外部 PostgreSQL

在 `.env` 中设置：

```bash
DATABASE_URL=postgresql://user:password@host:5432/dbname
```

要求：后端容器能访问到该 `host`（同网络/同 VPC/或使用 `host.docker.internal`
等方案）。

## 常用命令

```bash
# 查看状态
docker compose ps

# 看日志
docker compose logs -f

# 停止
docker compose down

# 更新代码后重建
git pull
docker compose up -d --build
```

## FAQ / 故障排查（最常见）

### 1) macOS 下 SQLite“Device or resource busy”

如果你使用了 bind mount（默认是），且 SQLite
文件被其它进程占用，可能出现该问题。处理思路：

1. 停止所有相关容器：`docker compose down`
2. 确认没有进程占用 `ELIZABETH_DATA_DIR` 下的 `elizabeth.db`

（Docker 默认配置已将 SQLite journal mode 设为更稳的 `delete`，见
`docker/backend/config/backend.yaml`。）

### 2) 如何重置数据？

SQLite 默认数据在你配置的 `ELIZABETH_DATA_DIR` / `ELIZABETH_STORAGE_DIR`
目录下；确认不需要后再删除对应目录即可。

```bash
docker compose down
rm -rf docker/backend/data docker/backend/storage
docker compose up -d --build
```

## 下一步

- 生产部署与云平台：`DEPLOYMENT.md`
- API 说明：`API_GUIDE.md`
- WebSocket 协议：`WEBSOCKET_GUIDE.md`
