# Cloudflare 部署可行性调研与执行方案

> 状态：**调研结论 + 待评审方案**，未实施任何代码变更。
> 调研日期：2026-09-04。基于 `crates/board` @ `1.6.0`（16,876 行 Rust / 100 文件 / 39 个 HTTP 路由）与 `web` 静态导出产物（375 文件 / 15MB）。
> 所有 Cloudflare 侧数字均引自官方文档，见文末《参考来源》。

---

## 0. 结论先行

1. **可行，但不是"部署"级别的工作，是"移植"级别的工作。** 直接把现有 Rust 二进制推上 workerd 不可能：依赖树里有 `sqlx / tokio(net,fs) / aws-lc-sys / ring / quanta / mio / socket2 / hyper / reqwest / rust-embed`，全部无法编译到 `wasm32-unknown-unknown`。
2. **域模型极其适配 Cloudflare。** 本项目是"房间为中心"的，11 张表里 9 张是 room-scoped，WebSocket 状态是进程内 per-room `HashMap`。这正好是 Durable Object 的教科书形态：**一个房间 = 一个 DO = 一份 SQLite + 一组休眠 WebSocket**，事务语义还能原样保留（`storage().transaction()`）。
3. **前端可以今天就迁走，零风险。** Next.js 已是 `output: "export"`，15MB/375 文件远低于 Workers 静态资产上限（Paid 100,000 文件 / 25MiB 每文件），且静态资产命中**不计费**。
4. **文件存储迁 R2 是纯赚。** R2 出网流量免费——对一个文件分享产品来说这是最大单项收益，且现有 `storage/backend.rs` 已经预留了 S3 抽象（虽然是死代码，见 §1.4）。
5. **推荐路径是分阶段的，不是一次性重写**：
   - **Phase 1（1–2 天，立即可做）**：前端 → Workers 静态资产；把 `tokio::fs` 直写收敛成真正的 `StoragePort`，接上 R2/S3。这两件事在现有 VPS 部署上同样收益，不产生沉没成本。
   - **Phase 0.5（可选旁路，3–5 天）**：Cloudflare Containers 直接搬现有镜像，最快脱离 VPS。代价：需 Workers Paid、磁盘非持久（SQLite 不可用，必须换托管 Postgres）、成本≈小 VPS、不是真 serverless。
   - **Phase 2（4–8 周，终局）**：workers-rs 移植，`RoomDO(SQLite+WS) + D1 注册表 + R2 + Cron/Alarm`。
6. **必须先做 2–3 天 spike 定 go/no-go**，五条硬指标见 §6.1。不通过就退到 A3（Workers + Hyperdrive + 托管 Postgres）或长期停在 Phase 0.5。
7. **不要放弃单容器自托管产品线。** Docker Hub 镜像是本产品的交付形态之一（README 首屏）。正确做法是**加一个 adapter，不是换实现**：纯域核心共享，`repository / storage / ws / config / scheduler` 四类适配器双实现。这条约束反过来决定了 Phase 2 必须以"端口化重构"开场。

成本预期：一个中小部署 **≈ $5–6/月**（Workers Paid 基础费 + 少量 R2 存储），出网免费；对照 VPS 省下的是运维、带宽和单点。免费额度能否兜住取决于 wasm 体积（<3MB gz）与 Argon2 的 10ms CPU 上限，**大概率不行**，见 §7-P1-10。

---

## 1. 现状盘点（代码事实）

### 1.1 交付形态

| 项 | 现状 | 关键文件 |
| :-- | :-- | :-- |
| 后端 | Rust + axum 0.8，单二进制单端口，承载 REST + WS + 内嵌 SPA | `crates/board/src/lib.rs:150` |
| 前端 | Next.js 16 静态导出 → `web/out` → `rust-embed` 编进二进制 | `web/next.config.mjs:13`、`crates/board/src/lib.rs:272` |
| 数据库 | `sqlx::AnyPool`，SQLite 默认 / Postgres 可选，双套 migrations（8 + 9 个文件） | `crates/board/src/db/mod.rs:12` |
| 文件 | 本地文件系统直写，`storage/rooms/{room}/...` | `crates/board/src/handlers/content/upload.rs:9` |
| 实时 | 进程内 `ConnectionManager`：`room_name -> Vec<connection_id>` + mpsc sender | `crates/board/src/websocket/connection.rs:45` |
| 定时任务 | 自研 `TaskScheduler`（tokio interval）：房间 GC / token 清理 / 上传清理 / 限流清理 | `crates/board/src/lib.rs:342` |
| 鉴权 | 房间 JWT（**HS256**）+ Argon2id 房间密码 + `X-Elizabeth-Admin-Token` | `crates/board/src/services/token.rs:78`、`room_password.rs:31` |
| 限流 | `tower_governor` + `SmartIpKeyExtractor`（进程内令牌桶） | `crates/board/src/middleware/rate_limit.rs` |

### 1.2 数据层形态（决定迁移难度的关键）

- **全部是运行时 SQL**：`sqlx::query(73) / query_as(7) / query_scalar(33)`，**零 `query!` 宏**。SQL 是纯文本 → 换驱动时 SQL 本体大部分可复用，这是好消息。
- **63 处显式事务**（`pool.begin()` 计 21 个调用点，含 commit/rollback 共 63 处引用）。分布在全部 10 个 repository。
- **11 张表**：`rooms` / `token_blacklist` 为全局；`room_contents / room_tokens / room_refresh_tokens / room_upload_reservations / room_roles / room_chunk_uploads / room_access_logs / file_download_policies / file_access_codes` 均为房间维度。
- **跨边界不变量**：`rooms.current_size`、`rooms.current_times_entered`、`rooms.roles_version` 与房间内表在**同一事务**里读改写（`room_access_repository.rs:101`、`room_upload_reservation_repository.rs:136`、`room_content_repository.rs:298`）。这条决定了 §4 的 A2 方案必须把 `rooms` 行放进 DO 内。

### 1.3 上传链路

- 普通上传：`multipart/form-data`，默认体上限 100MB（`constants.rs: MAX_MULTIPART_BODY_SIZE`）。
- 分块上传：`MAX_CHUNK_SIZE = 1MB`，分块落到 `storage_root/.chunks/{reservation_id}/chunk_N`，完成时合并成 `merged_file` 再**原子 rename** 进房间目录（`chunk_temp_storage.rs:18`）。
- 下载：`tokio::fs::File` + `ReaderStream` 全量流式，**无 Range 支持**（`handlers/content/download.rs:138`）。

### 1.4 已存在但未接线的抽象

`crates/board/src/storage/backend.rs` 定义了完整的 `StorageBackend` trait + `OpendalBackend`（FS/S3），注释里甚至写了 "Cloudflare R2"。但：

```
$ grep -rn 'OpendalBackend|StorageBackend' src --exclude-dir=storage
(无结果)
```

`opendal` 出现在 `Cargo.toml [workspace.metadata.cargo-shear] ignored` 名单里 —— 即**明知未使用**。handlers 全部直接调 `tokio::fs`。所以"存储抽象已就绪"是假象，**Phase 1 的第一件事就是把它接线**。

---

## 2. Cloudflare 侧能力与硬限制（核对过的数字）

### 2.1 Workers 运行时

| 限制 | Free | Paid | 对本项目的影响 |
| :-- | :-- | :-- | :-- |
| CPU / 请求 | **10 ms** | 30 s（可调至 5 min） | Argon2id 在 Free 下必然超限 |
| 内存 / isolate | 128 MB | 128 MB | Argon2 默认 m=19MiB 可容纳 |
| 脚本体积（gzip） | **3 MB** | **10 MB** | wasm 体积是最大不确定项；`rust-embed` 的 15MB 前端**必须先摘掉** |
| 请求体 | 100 MB（Pro 同）| Business 200MB / Ent 可至 5GB | 大文件必须直传 R2，不能过 Worker |
| 响应体 | 无强制上限 | 同 | 下载没问题 |
| 子请求 / 调用 | 50 | 10,000 | 分块合并类操作要留意 |
| 并发出网连接 | 6 | 6 | 批量 R2 操作需自己限并发 |
| Cron Triggers | 5 个（10ms CPU）| 250 个（<1h 间隔 30s CPU） | GC 任务在 Free 下不可用 |
| 静态资产 | 20,000 文件 | 100,000 文件；25 MiB/文件 | 375 文件 / 15MB，绰绰有余 |
| 请求数 | 100k/天 | 按量 | — |

**静态资产命中不触发 Worker 调用即不计费**（"Requests are only billable if a Worker script is invoked"）。

### 2.2 Durable Objects（本方案的核心）

- **Free 计划可用**（2025-04-07 起），但**仅 SQLite 后端**。
- SQLite 存储：Paid **10 GB/对象**，账户不限；Free 账户合计 5 GB。
- CPU：默认 **30 s/请求**，且"每个进入的 HTTP 请求或 WebSocket 消息都会把可用 CPU 重置回 30s"。
- WebSocket Hibernation：`acceptWebSocket()` 后对象可被逐出内存而连接保持；**休眠期不计 duration 费用**；ping/pong 由运行时自动应答且不唤醒对象。
- 计费：Paid 含 1M 请求 + 400k GB-s，超出 $0.15/M 请求、$12.50/M GB-s；**入站 WS 消息按 20:1 折算**（100 条 = 5 请求）。
- 行级计费：Free 5M 读/天、100k 写/天；Paid 含 25B 读 / 50M 写每月。
- 单个 DO **单线程，软上限 ~1000 req/s**；超载排队后报 overloaded。
- 部署新版本会**断开所有 WebSocket**。
- 单对象出网并发同样是 6。

### 2.3 D1

- 库大小 10 GB（Free 500 MB）；每次 Worker 调用 1000 次查询（Free 50）。
- **不支持交互式事务**。SQL 里写 `BEGIN` 会直接报错，官方错误信息明确指向 `state.storage.transaction()`。唯一原子机制是 `db.batch([...])`（一次往返、顺序自动提交、失败整批回滚）。
- 后果：**batch 内不能读**。所有校验必须在 batch 之前的读阶段完成，ID 需预生成，计数器增减必须写成 SQL 表达式而非应用层计算。
- 每库背后是一个 DO，**串行处理查询**：1ms 查询 ≈ 1000 qps，100ms 查询 ≈ 10 qps。
- 其他：绑定参数 100 个、单语句 100 KB、单行/BLOB 2 MB、单表 100 列。

### 2.4 R2

- 免费额度：10 GB-月存储、1M Class A、10M Class B；**出网永久免费**。
- 付费：$0.015/GB-月、Class A $4.50/M、Class B $0.36/M。
- `PutObject` / `CreateMultipartUpload` / `UploadPart` / `CompleteMultipartUpload` 都是 **Class A**；`GetObject` / `HeadObject` 是 Class B；`DeleteObject` / `AbortMultipartUpload` 免费。
- 对象上限 ~5 TiB；单次 PUT ≤ 5 GiB；分片最多 10,000。
- **分片必须 ≥5 MiB，且所有非尾片长度完全相等**（R2 强于 S3 的约束，在 `CompleteMultipartUpload` 时校验，失败信息 "All non-trailing parts must have the same length"）。→ 现有 1MB 分块协议不兼容。
- 同一 key 并发写 **1 次/秒**；未完成的 multipart **7 天自动 abort**。
- Workers 绑定支持 multipart 全套 + Range 读 + 条件请求 + 自定义元数据。
- 预签名 URL：GET/HEAD/PUT/DELETE，有效期 1s–7 天，**只能用 `<account>.r2.cloudflarestorage.com`，不支持自定义域**；浏览器直传需要配 CORS。

### 2.5 workers-rs（Rust on workerd）

- 当前版本 **`worker` 0.8.5**（2026-06-12 发布，迭代活跃）。目标 `wasm32-unknown-unknown`。
- **无 tokio / async_std**；`async/await` 语法可用（wasm-bindgen-futures 桥接，整个 handler 变成一个 JS Promise，单线程 `spawn_local`）。`tokio::sync` 这类运行时无关的部分仍可用。
- **axum 可用**：`http` feature 让 fetch handler 直接收 `http::Request<worker::Body>`，官方仓库有 `examples/axum`，入口即 `router(env).call(req).await`。
- **!Send 摩擦**：JS 对象不实现 `Send`，axum 强制要求 `Send`。官方给了三件套：`send::SendFuture`、`send::SendWrapper`、`#[worker::send]` 宏。调试靠 `#[debug_handler]`。
- **Durable Object 支持完整**（已核对 `worker/src/durable.rs` API 面）：
  `accept_web_socket` / `accept_websocket_with_tags` / `get_websockets` / `get_websockets_with_tag` / `get_tags` / `set_websocket_auto_response` / `storage().transaction()` / `storage().sql()` / `set_alarm`。
- 绑定齐全：`KvStore / ObjectNamespace / Bucket(R2) / D1Database / Queue / Hyperdrive / RateLimiter / SecretStore / Fetcher`。
- 成熟度警告：**D1 binding 标注 alpha**（`d1` feature 门控）、Queues beta、**RPC experimental**（函数参数/返回值/stub 转发"未支持或未测试"）→ **Worker↔DO 通信走 HTTP/fetch，不要用 RPC**。
- 辅助能力：`worker::signals::is_near_cpu_limit()` 可优雅退让；`worker-build --panic-unwind` 让 panic 转成 JS 异常而不是整个实例挂掉。
- `examples/` 里有 `axum / kv / queue / tokio-postgres / fetcher / rpc-*`，**没有** DO / WebSocket / R2 / D1 的独立示例 → 这几块需要自己按 docs.rs 摸，是主要的未知成本来源。

### 2.6 其他

- **Hyperdrive**：Free/Paid 都可用，支持任意 Postgres/MySQL（含 Neon、Supabase、PlanetScale），托管连接池 + 默认开启查询缓存。Rust 有 `Hyperdrive` 绑定，workers-rs 也有 `tokio-postgres` 示例 → **Workers 里连 Postgres 是走通的路**。
- **Containers**：需 Workers Paid。实例规格 `lite`(1/16 vCPU, 256MiB, 2GB) → `standard-4`(4 vCPU, 12GiB, 20GB)；镜像必须 `linux/amd64`；**磁盘全部 ephemeral**（重启回到镜像初态，快照"coming soon"，可用 FUSE 挂 R2 但无 SSD 性能）；`sleepAfter` 默认 10 分钟，停机走 SIGTERM→最多 15min→SIGKILL；冷启动 1–3 s；**终端用户无法发起非 HTTP 的 TCP/UDP**（必经 Worker）。计费 10ms 粒度，Paid 含 25 GiB-h 内存 / 375 vCPU-min / 200 GB-h 磁盘，出网 NA/EU $0.025/GB（含 1TB）。
- **Rate Limiting binding**：`simple` 一种类型，`period` 只能 10 或 60 秒，**per-colo 且刻意不精确**（"permissive, eventually consistent"），dashboard 不可见。
- **Pages vs Workers**：Pages 上用 DO 需要额外起一个 Worker 承载，官方措辞是"Using Durable Objects with Workers is simpler and recommended"；Cron Triggers / Rate Limiting binding / Queue consumers / Workers Logs / 子路径资产 在 Pages 上都没有。→ **本项目应直接用 Workers 静态资产，不要用 Pages。**

---

## 3. 依赖黑名单（编译期即失败）

从 `Cargo.lock` 实测存在、且无法进 `wasm32-unknown-unknown` 的：

| 依赖 | 版本 | 为什么不行 | 替代 |
| :-- | :-- | :-- | :-- |
| `sqlx` | 0.8 | 需要 tokio 网络栈 + 原生驱动 | D1 binding / DO `sql()` / `tokio-postgres`+Hyperdrive |
| `aws-lc-sys` / `aws-lc-rs` | 0.41 / 1.17 | C 代码（`jsonwebtoken` 的 backend） | HS256 用 `hmac` + `sha2`（纯 Rust，约 30 行）或 WebCrypto |
| `ring` | 0.17 | C/汇编 | 同上 |
| `quanta` | 0.12 | 高精度时钟系统调用（`governor`→`tower_governor` 依赖） | Rate Limiting binding / DO 计数器 |
| `mio` / `socket2` / `hyper` / `reqwest` | — | 系统 socket | `worker::Fetch` / `worker::Socket` |
| `tokio` (net/fs/signal/process) | 1.x | 无 OS | 无对应物，逻辑需重写 |
| `rust-embed` | 8 | 可编译，但会把 15MB 前端塞进 wasm | Workers 静态资产 |
| `fs4` | 1 | 文件锁（`configrs` 依赖） | 配置改走 env/bindings/Secrets Store |
| `config` | 0.15 | 文件系统配置加载 | 同上 |
| `tower-http` compression | 0.6 | brotli 可编译但无意义 | Cloudflare 边缘自动压缩 |

**大概率可用但需 spike 验证**：`serde / serde_json / chrono`(需 `wasmbind`，默认开) / `uuid`(需 getrandom 的 `js`/`wasm_js` backend + `RUSTFLAGS`) / `argon2`(纯 Rust，成本在 CPU) / `sha2` / `hex` / `regex` / `bitflags` / `utoipa` / `axum` / `multer`(axum Multipart 底座)。

**几乎零改动可复用**（这是项目的资产）：`board-protocol` 全部 DTO/模型、`board::authz`（纯授权引擎，`authorize()` 不做 I/O）、`board::validation`、`services` 里的纯业务规则。按 `docs/ARCHITECTURE.md` 的分层，这部分本来就设计成不依赖 HTTP/DB。

---

## 4. 路线对比

| | A1 Workers+D1 | **A2 Workers+RoomDO** | A3 Workers+Hyperdrive+PG | B Containers | C 只迁前端 |
| :-- | :-- | :-- | :-- | :-- | :-- |
| 后端代码改动 | 大 | 大 | 中 | 极小 | 无 |
| SQL 可复用度 | 高（去掉事务） | **很高（含事务）** | 很高 | 100% | — |
| 事务语义 | ❌ 需重构 63 处 | ✅ `storage().transaction()` | ✅ 原生 | ✅ | — |
| WS 架构 | DO（仅连接） | **DO（连接+数据同地）** | DO（仅连接） | 单实例内存 | — |
| 数据持久 | D1 | DO SQLite + D1 投影 | 外部 Postgres | 外部 Postgres（磁盘非持久） | — |
| 外部依赖 | 无 | **无** | Neon/Supabase | Neon/Supabase | — |
| 真 scale-to-zero | ✅ | ✅ | ✅ | 部分（10min 后睡） | — |
| Free 计划可跑 | 视体积/CPU | 视体积/CPU | 视体积/CPU | ❌ 需 Paid | ✅ |
| 写吞吐瓶颈 | **全局单库串行** | 按房间分片 | Postgres | 单实例 | — |
| 工期 | 4–7 周 | **4–8 周** | 3–5 周 | 3–5 天 | 1–2 天 |
| 主要风险 | 事务重构改变语义 | DO 编程模型 + D1 投影一致性 | 多一个供应商 + 冷连接延迟 | 成本≈VPS、WS 被睡眠打断 | 无 |

### 4.1 推荐终局：A2

```
                         ┌──────────────────────────────────────┐
   Browser ─────────────▶│  Worker: elizabeth-edge              │
        │                │   assets: web/out (静态命中→不计费)   │
        │                │   main:   Rust/WASM (axum + utoipa)  │
        │                │   bindings: ROOM_DO, DB(D1), R2,     │
        │                │             RATE_LIMITER, secrets    │
        │                └──┬────────────┬──────────────┬───────┘
        │                   │ fetch      │ D1           │ R2 binding
        │       ws upgrade  │ (非 RPC)   │ (仅注册表)   │ (Range/multipart)
        │            ┌──────▼──────────┐ │              │
        └───────────▶│  RoomDO[房间名] │ │              │
          (WS 直连)  │  ├ SQLite: 房间 │ │        ┌─────▼─────┐
                     │  │  全部业务表   │ │        │    R2     │
                     │  │  + rooms 行  │ │        │  对象存储  │
                     │  ├ 休眠 WS 集合 │ │        └───────────┘
                     │  └ alarm: 过期  │ │              ▲
                     └─────────┬───────┘ │              │ 预签名 PUT
                               │ 投影写   │              │ (浏览器直传)
                               └─────────▶│         Browser
                                          │
                     Cron Trigger ────────┘  (兜底 GC / 孤儿对象清理)
```

**为什么是这个形状**

- 9/11 张表是 room-scoped，`rooms` 行又与房间内数据存在同事务不变量（§1.2）→ **把 `rooms` 行也放进 DO**，DO 就成为该房间的完整数据库，现有 SQL（含 `room_id` 列、含事务）几乎原样可用。
- `ConnectionManager` 的 `room_name -> connections` 语义与 DO 的"按名寻址单实例"逐字对应；HTTP handler 里 10 处 `broadcaster` 调用变成对同一个 DO stub 的 fetch。
- 房间过期不再需要全表扫描：每个 RoomDO 在 `expire_at` 设一个 alarm，到点自清自删。Cron 只做兜底。
- 删房间 = 删 DO + 删 R2 前缀，天然干净。

**D1 只保留一张投影表**，供 admin 列表 + GC 候选扫描 + 房间名唯一性：

```sql
CREATE TABLE room_registry (
  name TEXT PRIMARY KEY,
  slug TEXT NOT NULL UNIQUE,
  status INTEGER NOT NULL,
  expire_at TEXT,
  updated_at TEXT NOT NULL,
  size_bytes INTEGER NOT NULL DEFAULT 0
);
```

投影由 DO 在提交后写入 → **最终一致**。规则必须写死：*投影只用于候选筛选与展示，任何授权/配额判定一律回 DO 复核*。房间名唯一性用 D1 的 `PRIMARY KEY` 抢占 + DO 内二次确认（先 D1 INSERT 占名，再 DO 初始化，失败则补偿删除）。

---

## 5. 端口化重构（Phase 2 的前置，且对现有部署同样有益）

目标：域核心与运行时解耦，native 与 worker 两套 adapter 并存。这不是为 Cloudflare 额外付的税——它同时消掉了 §1.4 那个"假抽象"，也让 `services` 层第一次真正可单测。

```
crates/
  board-protocol/       # 不变：DTO / 模型 / Capability
  board-core/    (新)   # 纯域：authz + validation + 业务规则 + 端口 trait 定义
  board-native/  (新)   # adapter: sqlx / tokio::fs|opendal / 进程内 WS / TaskScheduler
  board-worker/  (新)   # adapter: DO sql() / R2 / DO WS / Cron+Alarm / RateLimiter
  board/                # 保留为 native 可执行入口（现有 CLI/Docker 不变）
```

端口清单（`board-core::ports`）：

| 端口 | native 实现 | worker 实现 |
| :-- | :-- | :-- |
| `RoomStore` / `ContentStore` / `TokenStore` / `PolicyStore` / `RoleStore` | sqlx + AnyPool | DO `storage().sql()` |
| `UnitOfWork`（事务边界） | `pool.begin()` | `storage().transaction()` |
| `RoomRegistry`（跨房间查询） | 同一 DB 直接查 | D1 |
| `ObjectStore` | opendal FS/S3 | R2 binding |
| `Broadcast` | `ConnectionManager` | DO stub fetch |
| `Clock` / `Rng` / `PasswordHasher` / `JwtCodec` | chrono / uuid / argon2 / hmac | 同（wasm 版）|
| `Limiter` | tower_governor | RateLimiter binding |
| `Scheduler` | TaskScheduler | Cron Trigger + DO alarm |
| `ConfigSource` | configrs（文件） | env + bindings + Secrets |

**关键技术细节（会咬人）**：端口 trait 在 wasm 侧的 future 不是 `Send`。统一用 `#[async_trait(?Send)]`（或 `trait-variant` 生成两版），**不要**在 core 里要求 `Send + Sync`。同时 axum handler 需要 `#[worker::send]` 包裹。这两处不统一，会在移植后期爆发成大面积签名返工——**必须在 Phase 2 第一天就定下来**。

---

## 6. 分阶段执行计划

### 6.1 Phase 0：Spike（2–3 天）— go/no-go 门禁

在一个丢弃分支上做，只求信号，不求质量。

| # | 验证项 | 通过标准 | 不通过的退路 |
| :-- | :-- | :-- | :-- |
| S1 | `axum + utoipa + serde + chrono + argon2 + hmac` 编到 wasm 并 `wrangler deploy` | gzip **< 8 MB**（Paid 上限 10MB，留冗余）；启动 < 1s | 砍 utoipa（OpenAPI 改构建期生成）；再不行退 A3/B |
| S2 | 3 个代表性路由跑通：`GET /rooms/{n}/contents`、`POST /rooms/{n}/messages`、`POST .../tokens` | 含 `#[worker::send]` + `?Send` 端口，能编译能跑 | 退 A3 或改 TS 重写后端 |
| S3 | RoomDO：`sql().exec()` + `transaction()` + WS hibernation echo + `set_alarm` | 事务回滚可验证；WS 断连重连正常 | 退 A1（D1 + batch），事务重构成本翻倍 |
| S4 | `multipart/form-data` 在 wasm 里解析（axum `Multipart`/multer） | 10MB 文件可解析落 R2 | 全面改预签名直传（推荐无论如何都做） |
| S5 | Argon2id hash + verify 的 CPU 实测 | < 200 ms（Paid 30s 内） | 换 PBKDF2-HMAC-SHA256（WebCrypto）+ 密码重哈希迁移 |

同时产出一份《不可移植清单》—— spike 中实际撞到的编译错误全记下来，它比任何预估都准。

### 6.2 Phase 1：立刻可做、双向受益（1–2 天）

1. **前端上 Workers 静态资产**。`wrangler.jsonc` 里 `assets.directory = web/out`。
   ⚠️ **不要用 `not_found_handling: "single-page-application"`**：它把未命中路径一律回 `/index.html`，而本项目的房间页是静态导出的 `_.html` 模板（`crates/board/src/lib.rs:305` 的三步 fallback）。`/myroom` 拿到 `index.html` 会 hydrate 成首页。
   正确做法：`not_found_handling: "none"` 让未命中请求进入 Worker，由 Worker 复刻现有 fallback（`api/*` → JSON 404；命中资产 → 返回；根 → `index.html`；其余 → `_.html`）。
   ⚠️ 还要注意：当 `not_found_handling` 被配置且 compat date ≥ 2025-04-01 时，**浏览器导航请求默认跳过 Worker**（靠 `Sec-Fetch-Mode: navigate` 判定）。必须用 `run_worker_first` 显式接管或依赖 `none` 语义，**此处需实测确认**，是最容易踩空的一步。
2. **把 `StoragePort` 真正接线**（消灭 §1.4 的死代码）：handlers 里 5 处 `tokio::fs` 收敛到端口后面，opendal S3 指向 R2。这一步在现有 VPS 上就能立刻用 R2 当存储、吃掉零出网费。
3. **下载改走 Range**：R2/opendal 都支持，顺手补上现在缺失的断点续传。
4. **OpenAPI 移到构建期**：`build_api_router` 现在每次启动都 `to_string_pretty` 整份 OpenAPI（`lib.rs:232`），Workers 只有 1s 启动预算，先在 native 侧改成 build.rs 产物。

Phase 1 结束时：VPS 上仍是同一个二进制，但文件已在 R2、前端已在 Cloudflare 边缘、存储抽象已真实存在。**即使 Phase 2 永不发生，这些也不浪费。**

### 6.3 Phase 0.5（可选旁路）：Containers 过渡（3–5 天）

想尽快下掉 VPS 又不想等 4–8 周就走这条。

- 复用现有 `Dockerfile.backend`（已是多架构，需固定 `linux/amd64`）。
- **必须换库**：磁盘 ephemeral → SQLite 会在每次重启/睡眠后归零。换托管 Postgres（Neon/Supabase 免费档），`DATABASE_URL` 一行配置，`migrations_pg` 已就绪。
- **必须先做 Phase 1 的 R2 接线**，否则文件同样随磁盘蒸发。
- WS 正确性依赖"所有流量落到同一个实例"：用固定 DO id（如 `getContainer(env.C, "singleton")`）保证单实例语义 —— 这等于放弃水平扩展，但与今天的语义一致。
- 代价清单：Workers Paid $5/月起 + 容器计费（内存/磁盘按 provisioned 计，CPU 按实际）；`sleepAfter` 默认 10min，睡眠会断 WS；冷启动 1–3s；Workers + DO 用量另计。**长期成本不比小 VPS 便宜，换来的是免运维和全球接入。**

### 6.4 Phase 2：workers-rs 移植（4–8 周）

按依赖顺序，每步都有独立验证门禁：

| 步 | 内容 | 门禁 |
| :-- | :-- | :-- |
| 2.1 | 拆 `board-core`（纯域）+ 端口 trait；native adapter 迁入 `board-native` | `cargo test --workspace` 全绿，行为零变化；native 二进制功能不变 |
| 2.2 | `board-core` 单独跑 `cargo check --target wasm32-unknown-unknown` | 纯域必须先能过 wasm，这是最便宜的早期信号 |
| 2.3 | RoomDO 骨架：SQLite migrations（DO 内 `user_version` 自迁移）+ 事务 + 一张表打通 | DO 内 CRUD + 回滚测试通过（workerd/miniflare） |
| 2.4 | repository 逐个搬进 DO adapter（10 个，建议顺序：room → content → token → refresh → role → policy → access → reservation → chunk → lifecycle） | 每个 repo 一套 workerd 集成测试 |
| 2.5 | WS：DO hibernation + `accept_websocket_with_tags` 承接房间订阅；HTTP handler 的 10 处广播改 DO fetch | 现有前端不改代码即可连上；重连/断线用例通过 |
| 2.6 | 上传：预签名直传 R2 + `POST /commit` 落元数据；分块协议分片改 ≥5MiB 等长 | 100MB+ 文件上传成功；断点续传与清理正常 |
| 2.7 | D1 注册表 + admin 端点 + Cron 兜底 GC + DO alarm 过期 | 房间过期/GC 端到端；投影与 DO 一致性用例 |
| 2.8 | 鉴权：HS256 换纯 Rust hmac；Argon2 参数实测定档；admin token 走 Secrets | 现有 token 必须继续验签通过（不改密钥语义） |
| 2.9 | 限流：RateLimiter binding；IP 取 `CF-Connecting-IP` 替 `SmartIpKeyExtractor` | 429 行为与现状可比 |
| 2.10 | 数据迁移：`app.db` → 逐房间导入 DO；`storage/rooms/**` → R2（rclone） | 抽样对账：房间数、内容数、文件 hash |
| 2.11 | 双轨观察期：native（VPS/Docker）与 Worker 并行，只读流量镜像比对 | 差异归零后切流 |

**CI 必须同时守两个目标**：`cargo clippy --workspace -- -D warnings` 与 `cargo check -p board-worker --target wasm32-unknown-unknown`。少一个，双 adapter 会在两周内腐烂。

---

## 7. 难点与坑点清单

### P0 — 阻断级

| # | 坑 | 说明 | 对策 |
| :-- | :-- | :-- | :-- |
| 1 | 依赖树整条不可用 | `sqlx / tokio(net,fs) / aws-lc-sys / ring / quanta / mio / socket2 / hyper / reqwest` | §3 黑名单逐项替换；spike 先量 |
| 2 | `rust-embed` 把 15MB 前端塞进 wasm | Paid 脚本上限 10MB gz → **直接超限** | Phase 1 先切静态资产（这是 Phase 2 的硬前置） |
| 3 | D1 无交互事务 | 63 处 `.begin()`；batch 内不能读 → 读改写不变量失效 | 选 A2（DO 事务可用）；若走 A1，`rooms.current_size` 这类必须改乐观并发（`WHERE version = ?`）或移入 DO |
| 4 | 上传体积 | Worker 请求体 100MB(Free/Pro)，超了 413 | 预签名直传 R2，Worker 只签名+记账 |
| 5 | R2 分片约束 | 必须 ≥5 MiB 且非尾片**等长**；现状 `MAX_CHUNK_SIZE = 1MB` | 分块协议改 8 MiB 固定片（8MiB × 10000 ≈ 80GB 上限，够用）；前端 uploader 同步改 |
| 6 | 本地暂存 + 原子 rename | `.chunks/` → `merged_file` → rename，Workers 无文件系统也无 rename | 直接用 R2 multipart（分片即 part，complete 即"合并"），删掉暂存目录概念 |
| 7 | 进程内 WS 广播 | `ConnectionManager` 是 per-isolate 的，Workers 里天然多实例 | 每房间一个 DO 持有连接；广播全部经 DO |
| 8 | 配置体系 | `configrs` 依赖 `config`(读文件) + `fs4`(文件锁) | `ConfigSource` 端口；worker 侧用 env/bindings/Secrets Store |

### P1 — 高风险 / 高成本

| # | 坑 | 说明 | 对策 |
| :-- | :-- | :-- | :-- |
| 9 | axum 的 `Send` 冲突 | JS 对象非 `Send`，axum 强制 `Send`；错误信息形如 `Rc<RefCell<wasm_bindgen_futures::Inner>> cannot be sent between threads` | `#[worker::send]` + `SendWrapper` + `SendFuture`；端口 trait 统一 `?Send`；`#[debug_handler]` 定位 |
| 10 | Argon2 CPU 预算 | Free 10ms 必然超；Paid 30s 应该够但**没实测** | spike S5；不达标换 WebCrypto PBKDF2 + 渐进重哈希 |
| 11 | 启动时密码迁移无处安放 | `migrate_legacy_room_passwords` 现在跑在 `start_server`（`lib.rs:139`），Workers 无启动钩子 | 改成一次性 admin 端点 / Cron 单次任务，迁移完即删 |
| 12 | 静态资产路由语义 | SPA 模式回 `index.html` ≠ 项目需要的 `_.html`；导航请求默认跳过 Worker | `not_found_handling: "none"` + Worker 复刻 fallback；**必须实测** |
| 13 | 投影最终一致 | D1 `room_registry` 落后于 DO 真相 | 铁律：投影只做候选/展示；配额与授权回 DO 复核；抢名走 D1 主键 + 补偿删除 |
| 14 | workers-rs 成熟度 | D1 binding alpha、RPC experimental、DO/WS/R2 无官方示例 | Worker↔DO 一律 fetch 不用 RPC；给这块留 30% 工期缓冲 |
| 15 | 定时任务重构 | tokio interval → Cron（Free 10ms CPU 不够）+ DO alarm | 过期用 per-room alarm（更省更准）；Cron 只做孤儿对象兜底 |
| 16 | 限流降级 | `tower_governor` 不可用；binding 只有 10/60s 周期、per-colo、刻意不精确 | 粗粒度用 binding；`access_code_limiter` 这种要求准确的移进 DO 计数 |
| 17 | 单点吞吐 | 单 DO ~1000 req/s 单线程；D1 单库串行 | A2 天然按房间分片；避免把热路径塞进 D1 |

### P2 — 需注意

| # | 坑 | 对策 |
| :-- | :-- | :-- |
| 18 | 部署新版本断开所有 WS | 前端已有重连（`lib/hooks/use-websocket.ts`），需补"部署窗口"用例；用渐进部署 |
| 19 | 预签名 URL 不支持自定义域 | 上传走 `*.r2.cloudflarestorage.com`；下载走 Worker + R2 binding（保住鉴权与自定义域） |
| 20 | R2 同 key 1 写/秒 | 内容 key 用 `content_id`/uuid，不要用可覆盖的稳定名 |
| 21 | 未完成 multipart 7 天自动 abort | 上传预留 TTL 现为 1h，比 7d 短，无冲突；但清理任务要同时清 R2 侧残片 |
| 22 | `uuid` / `getrandom` 在 wasm | 需 `js`/`wasm_js` backend + `RUSTFLAGS`；lockfile 里同时存在 getrandom 0.2/0.3/0.4，需统一 |
| 23 | `chrono` 时钟 | 依赖 `wasmbind`（默认开），确认没有被 `default-features = false` 关掉 |
| 24 | 观测 | `tracing-subscriber` 换 Workers Logs / Tail Worker；`shadow-rs` 构建信息可保留 |
| 25 | Free 计划天花板 | 100k 请求/天、DO 5GB 存储、10万行写/天、10ms CPU | 生产上 Paid；Free 只当 preview 环境 |
| 26 | 双 adapter 维护成本 | 自托管镜像是产品形态，不能砍 | CI 双目标门禁 + 纯域共享率尽可能高（目标 >60% 代码零改动） |
| 27 | 测试栈分裂 | 纯域 `cargo test`；adapter 需 workerd/miniflare | 按 `.agents/skills/rust-testing` 保持 `src/tests/**` 镜像结构；worker 侧集成测试单独 job |

---

## 8. 成本模型（示例量级）

假设：100 房间/月、5,000 次上传、20,000 次下载、200,000 条入站 WS 消息、500,000 次 API 请求、20 GB 常驻存储。

| 项 | 用量 | 费用 |
| :-- | :-- | :-- |
| Workers Paid 基础 | — | $5.00 |
| Workers 请求 | 500k（含 10M/月） | $0 |
| 静态资产命中 | 不触发 Worker | **$0** |
| DO 请求 | WS 200k ÷ 20 + 连接/REST ≈ 60k（含 1M） | $0 |
| DO duration | 休眠期不计 | ≈$0 |
| DO SQLite 行 | 读 ~2M / 写 ~200k（含 25B/50M） | $0 |
| D1 | 注册表级用量 | $0 |
| R2 存储 | 20 GB（免 10 GB） | ~$0.15–0.30 |
| R2 Class A | 上传 5k × 3（multipart 三步）= 15k（免 1M） | $0 |
| R2 Class B | 20k（免 10M） | $0 |
| **出网流量** | 任意 | **$0** |
| 合计 | | **≈ $5.2–5.3/月** |

对照：Containers 路线除上述外，还要按 provisioned 内存/磁盘计费（`basic` 1GiB/4GB 常驻一整月约 $6–7，睡眠期不计），出网 $0.025/GB（含 1TB）。

**判断**：绝对金额与小 VPS 相当，价值在于**零出网费**（文件分享产品的核心成本项）、零运维、全球就近接入、天然多区域。

---

## 9. 决策点与放弃条件

**建议动作顺序**：Phase 1 立刻做（无悔） → Phase 0 spike（2–3 天买信息） → 按 spike 结果二选一（A2 全量移植 / Phase 0.5 过渡）。

**应当放弃 A2、退到 A3 或 B 的信号**：

- spike S1 wasm gzip > 9 MB 且砍掉 utoipa 后仍不达标；
- spike S3 在 DO SQLite 事务或 WS hibernation 上撞到 workers-rs 的未实现 API，且 upstream 无短期解；
- 2.4 步（repository 搬迁）实际速度低于每周 2 个 repo —— 说明总工期会失控到 12 周以上；
- 出现任何"为了绕开平台限制而放松安全约束"的诉求（例如为省 CPU 降 Argon2 参数）——这条是红线，宁可留在 VPS。

**永远不做的事**：为了上 Cloudflare 而删掉单容器自托管路径。CF 是**新增一个 target**，不是替换。

---

## 10. 待验证清单（本文档的不确定性）

以下判断是基于文档推理，**未在本仓库实测**，实施前必须验证：

1. `axum 0.8` + `worker 0.8.5 http` feature 在本项目 39 路由规模下的可编译性与体积。
2. `Multipart`（multer）在 wasm 下的行为与内存占用。
3. Argon2id 默认参数在 workerd 的实际 CPU 毫秒数。
4. Workers 静态资产在 `not_found_handling: "none"` 下，浏览器导航请求是否确实进入 Worker（文档表述随 compat date 变化，且有 `assets_navigation_*` 多个 flag）。
5. Durable Objects 在 **Free 计划**下的单请求 CPU 上限究竟是 10ms 还是 30s —— DO 限制页写 30s 未区分计划，Workers 限制页写 Free 10ms，两处口径不一致。生产按 Paid 规划可回避该问题。
6. `worker` crate 的 `SqlStorage` 在 `transaction()` 闭包内的可用性（API 存在，组合语义待验）。
7. 现有 HS256 token 在换用 `hmac`+`sha2` 后的逐字节兼容（应当兼容，需用现网 token 回归）。

---

## 参考来源

**Cloudflare 官方文档**
- [Workers 限制](https://developers.cloudflare.com/workers/platform/limits/)
- [Workers Rust 语言支持](https://developers.cloudflare.com/workers/languages/rust/)
- [Workers 静态资产](https://developers.cloudflare.com/workers/static-assets/) / [SPA 路由](https://developers.cloudflare.com/workers/static-assets/routing/single-page-application/) / [从 Pages 迁移](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/)
- [Durable Objects 限制](https://developers.cloudflare.com/durable-objects/platform/limits/) / [定价](https://developers.cloudflare.com/durable-objects/platform/pricing/) / [WebSockets 最佳实践](https://developers.cloudflare.com/durable-objects/best-practices/websockets/) / [Free 计划公告 (2025-04-07)](https://developers.cloudflare.com/changelog/post/2025-04-07-durable-objects-free-tier/)
- [D1 限制](https://developers.cloudflare.com/d1/platform/limits/) / [D1 Worker API](https://developers.cloudflare.com/d1/worker-api/d1-database/)
- [R2 限制](https://developers.cloudflare.com/r2/platform/limits/) / [定价](https://developers.cloudflare.com/r2/pricing/) / [Workers API](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/) / [Workers 分片上传](https://developers.cloudflare.com/r2/api/workers/workers-multipart-usage/) / [预签名 URL](https://developers.cloudflare.com/r2/api/s3/presigned-urls/)
- [Containers 平台细节](https://developers.cloudflare.com/containers/platform-details/) / [限制](https://developers.cloudflare.com/containers/platform-details/limits/) / [定价](https://developers.cloudflare.com/containers/pricing/)
- [Hyperdrive](https://developers.cloudflare.com/hyperdrive/)
- [Rate Limiting binding](https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/)

**workers-rs**
- [cloudflare/workers-rs README](https://github.com/cloudflare/workers-rs) / [examples](https://github.com/cloudflare/workers-rs/tree/main/examples) / [axum 示例](https://github.com/cloudflare/workers-rs/blob/main/examples/axum/src/lib.rs) / [worker crate 文档](https://docs.rs/worker/latest/worker/)
- `worker` 版本与发布时间取自 crates.io API（0.8.5 / 2026-06-12）
- DO API 面核对自 [`worker/src/durable.rs`](https://raw.githubusercontent.com/cloudflare/workers-rs/main/worker/src/durable.rs)

**D1 事务限制的社区佐证**
- [D1 has no transactions — using client.batch()](https://firdausng.com/posts/d1-has-no-transactions-use-client-batch)
- [drizzle-orm #2463: Cloudflare D1 transaction not supported](https://github.com/drizzle-team/drizzle-orm/issues/2463)
- [How I cheated on transactions (event-driven.io)](https://event-driven.io/en/cloudflare_d1_transactions_and_tradeoffs/)

**R2 分片等长约束的佐证**
- [apache/arrow #41506](https://github.com/apache/arrow/issues/41506)（"Cloudflare R2 requires that every part be exactly equal"）
- [@tus/s3-store 文档](https://www.npmjs.com/package/@tus/s3-store)（`partSize == minPartSize` 的 R2 变通）

**本仓库证据**：`crates/board/src/{lib.rs,db/mod.rs,state.rs,storage/backend.rs,chunk_temp_storage.rs,constants.rs}`、`crates/board/src/websocket/*`、`crates/board/src/repository/*`、`crates/board/migrations/*.sql`、`web/next.config.mjs`、`Cargo.toml`、`Cargo.lock`。
