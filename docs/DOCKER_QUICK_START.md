# Elizabeth Docker 快速开始

目标：用 `docker compose` 在一台机器上跑起来（单容器，Rust 后端内嵌 SPA
前端），默认使用 SQLite；可选 PostgreSQL。

## TL;DR（默认 SQLite）

```bash
git clone https://github.com/YuniqueUnic/elizabeth.git
cd elizabeth

cp .env.docker .env
# 生产环境务必修改 JWT_SECRET（长度 >= 32）
${EDITOR:-nano} .env

# 创建挂载目录，并在 Linux 上把它们交给容器运行身份（见下方「运行身份」）
./scripts/docker_prepare_volumes.sh

docker compose up -d --build
docker compose ps
```

访问：

- Web 前端：`http://localhost:4092`
- OpenAPI UI：`http://localhost:4092/api/v1/scalar`
- OpenAPI JSON：`http://localhost:4092/api/v1/openapi.json`
- 健康检查：`http://localhost:4092/api/v1/health`

数据默认落盘在仓库目录（可通过 `.env` 覆盖）：

- `ELIZABETH_DATA_DIR`（SQLite DB 文件）
- `ELIZABETH_STORAGE_DIR`（上传文件）
- `ELIZABETH_BACKEND_CONFIG`（后端配置文件挂载路径）

## 运行身份与目录属主

镜像以 distroless 的 `nonroot` 用户运行（uid/gid `65532`），没有 shell，也不包含
`curl`：

- **Linux**：bind mount 保留宿主机属主，所以 `docker/backend/data` 与
  `docker/backend/storage` 必须属于
  `65532:65532`，否则容器无法写入、启动即失败。
  `./scripts/docker_prepare_volumes.sh` 会用 `sudo chown`
  完成这一步；若你自定义了 `ELIZABETH_UID` /
  `ELIZABETH_GID`，该脚本会读取并沿用同样的值。
- **macOS / Windows（Docker Desktop）**：bind mount 的属主由 Docker Desktop
  虚拟化，无需 chown，脚本会自动跳过。
- 健康检查由镜像内置的 `HEALTHCHECK` 完成（exec 形式调用 `/app/board health`）。
  不要改用 `docker run --health-cmd`：Docker CLI 总会把参数包进 `/bin/sh -c`，
  而该镜像没有 `/bin/sh`。
- 由于没有 shell，`docker exec -it <container> sh` 不可用；排障请看
  `docker compose logs`，或用 `docker exec <container> /app/board --help`。

## 开启管理面板 `/admin`（可选）

管理面板使用 bootstrap 管理员账号登录。在 `.env` 里设置一次：

```bash
ELIZABETH_ADMIN_USERNAME=admin       # 可选，默认 admin
ELIZABETH_ADMIN_PASSWORD=<强密码>     # 至少 12 个字符且不含空白
```

重启后首次启动会创建账号；此后在面板里修改密码会持久化到数据库，该环境变量
可以移除。忘记密码时也可以用 CLI 重置（见 `docs/cli.md` 第 7 节）。不设置
`ELIZABETH_ADMIN_PASSWORD` 则管理面板保持关闭。

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
