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

- `http://localhost:4092/`
- `http://localhost:4092/api/v1/scalar`

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

## 对象存储（S3 / R2，可选）

内容默认存放在本地文件系统；可切换到任意 S3 兼容对象存储（AWS S3、
MinIO、Cloudflare R2）。房间内容经服务端代理存取，桶地址与凭据不外露。

配置文件（`~/.config/elizabeth/config.yaml`）：

```yaml
app:
  storage:
    backend: s3 # fs（默认）| s3
    s3:
      endpoint: https://<account>.r2.cloudflarestorage.com
      bucket: elizabeth
      access_key_id: "<ACCESS_KEY_ID>"
      secret_access_key: "<SECRET_ACCESS_KEY>"
      region: auto # R2 填 auto；AWS 填如 us-east-1
```

或使用环境变量：

```bash
STORAGE_BACKEND=s3
STORAGE_S3_ENDPOINT=https://<account>.r2.cloudflarestorage.com
STORAGE_S3_BUCKET=elizabeth
STORAGE_S3_ACCESS_KEY_ID=<ACCESS_KEY_ID>
STORAGE_S3_SECRET_ACCESS_KEY=<SECRET_ACCESS_KEY>
STORAGE_S3_REGION=auto
```

说明：

- 切换后端只影响新写入的内容；已存量的本地文件继续按原路径读取，
  不需要迁移即可保持可下载。
- 分片上传的临时分片始终落在本地暂存目录（预留清理任务负责回收），
  最终文件才写入对象存储。
- 备份：`backend: fs` 时备份存储目录；`backend: s3` 时由桶的版本化 /
  复制策略负责，服务器侧只需备份数据库。

## 传输策略（proxy / presigned）

`storage.transfer` 决定内容如何进出对象存储：

- `proxy`（默认）：上传与下载一律经服务端代理，桶地址不出现在任何
  响应中，鉴权与下载策略完全在服务端执行。
- `presigned`（仅 `backend: s3` 可用）：服务端完成鉴权（房间 token、
  下载票据、访问码）与配额校验后，签发短时效预签名 URL：
  - 下载：`GET /api/v1/contents/{id}` 返回 302 跳转到直下 URL （`curl -L` /
    `wget` 透明跟随）；
  - 上传：prepare 响应携带逐文件直传 URL（`PUT`），直传完成后调用
    `POST /api/v1/rooms/{name}/contents/presigned-commit` 提交，服务端
    核对对象大小与清单一致后才落记录并核销预留。

相关配置：

```yaml
app:
  storage:
    backend: s3
    transfer: presigned
    presign_base_url: https://cdn.example.com # 可选：CDN / 自定义域名
    presign_ttl_seconds: 300 # 预签名有效期，默认 300 秒
```

环境变量：`STORAGE_TRANSFER=presigned`、`STORAGE_PRESIGN_BASE_URL`、
`STORAGE_PRESIGN_TTL_SECONDS`。

注意事项：

- 预签名 URL 的签发永远发生在鉴权之后，桶凭据不出现在任何响应中； 无有效 token /
  票据拿不到签名 URL。
- S3 SigV4 会把 Host 绑入签名；`presign_base_url` 替换域名仅在签名不 绑定 Host
  的部署（MinIO 配置 `domain`、透明签名代理）下可用，否则 保持缺省（使用 S3
  endpoint）。
- 历史 FS 内容（`/` 开头的 locator）无法预签名，下载自动回落为代理
  传输；`backend: fs` 时禁止启用 presigned（启动即失败）。

## 详细版本

- TLS/反向代理/云平台等：`DEPLOYMENT_FULL.md`
