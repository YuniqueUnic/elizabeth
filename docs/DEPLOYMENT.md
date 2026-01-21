# Elizabeth 部署指南（简版）

目标：给“第一次部署”的人一条最短路径；进阶细节下沉到 `DEPLOYMENT_FULL.md`。

## 推荐：Docker Compose 单机部署

```bash
git clone https://github.com/YuniqueUnic/elizabeth.git
cd elizabeth

cp .env.docker .env
# 生产环境务必修改 JWT_SECRET（长度 >= 32）
${EDITOR:-nano} .env

docker compose up -d --build
docker compose ps
```

默认对外暴露网关端口（见 `docker-compose.yml`）：

- `http://localhost:4001/`
- `http://localhost:4001/api/v1/scalar`

## SQLite / PostgreSQL 选择

- SQLite（默认）：无需额外服务，数据落在 `ELIZABETH_DATA_DIR`。
- PostgreSQL（可选）：使用 `docker-compose.postgres.yml` 或外部
  PostgreSQL，并设置 `DATABASE_URL=postgresql://...`。

```bash
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
```

## 数据持久化与备份（最小建议）

- SQLite：备份 `ELIZABETH_DATA_DIR` 下的 `elizabeth.db` +
  `ELIZABETH_STORAGE_DIR`。
- PostgreSQL：使用 `pg_dump` 备份数据库 + 备份 `ELIZABETH_STORAGE_DIR`。

## 详细版本

- TLS/反向代理/云平台等：`DEPLOYMENT_FULL.md`
