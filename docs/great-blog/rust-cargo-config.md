以下是 `.cargo/config.toml` 中比较有用的配置示例：

```toml
# ============================================================================
# Build 配置
# ============================================================================

[build]
# 并行编译任务数（默认为 CPU 核心数）
jobs = 8
# 增量编译（开发时推荐开启）
incremental = true
# 目标平台（可选）
# target = "x86_64-unknown-linux-gnu"

# ============================================================================
# Cargo 行为配置
# ============================================================================

[cargo]
# 默认使用 sparse 注册表协议（更快）
# registries-crates-io-protocol = "sparse"

[cargo-new]
# cargo new 时默认使用的 VCS
vcs = "git"
# 默认名称和邮箱
name = "Your Name"
email = "your.email@example.com"

# ============================================================================
# 编译器标志
# ============================================================================

[target.'cfg(all())']
rustflags = [
    # 开发阶段允许的 lint
    "-A", "unused-variables",
    "-A", "dead-code",
    "-A", "unused-imports",

    # 链接时优化相关
    # "-C", "link-arg=-fuse-ld=lld",  # 使用 lld 链接器（更快）

    # 调试信息
    # "-C", "debuginfo=1",  # 有限的调试信息（更快编译）
]

# macOS 特定配置
[target.x86_64-apple-darwin]
rustflags = [
    "-C", "link-arg=-fuse-ld=/usr/local/opt/llvm/bin/ld64.lld",
]

# Linux 特定配置
[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
]

# ============================================================================
# Profile 配置（编译优化）
# ============================================================================

[profile.dev]
# 开发模式优化级别（0-3，默认 0）
opt-level = 0
# 开启调试信息
debug = true
# 开启增量编译
incremental = true
# 依赖项的优化级别（加快运行速度但编译较慢）
[profile.dev.package."*"]
opt-level = 1

[profile.dev.build-override]
# 构建脚本的优化
opt-level = 3

# 快速开发 profile（介于 dev 和 release 之间）
[profile.dev-fast]
inherits = "dev"
opt-level = 1
debug = true

# Release 优化
[profile.release]
opt-level = 3
debug = false
lto = "thin"          # 链接时优化：thin（平衡）/ fat（最大）/ false
codegen-units = 1     # 更好的优化但编译慢
strip = true          # 移除符号（减小体积）
panic = "abort"       # panic 时直接退出（更小的二进制）

# 性能测试 profile
[profile.bench]
inherits = "release"
debug = true

# ============================================================================
# 注册源配置（加速依赖下载）
# ============================================================================

[source.crates-io]
replace-with = "ustc"  # 或 "tuna" / "sjtu" / "rsproxy"

# 中科大源
[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"

# 清华源
[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"

# 上海交大源
[source.sjtu]
registry = "https://mirrors.sjtug.sjtu.edu.cn/git/crates.io-index/"

# 字节跳动源
[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

# Git 依赖使用镜像
[source."https://github.com/rust-lang/crates.io-index"]
replace-with = "ustc"

# ============================================================================
# 网络配置
# ============================================================================

[net]
# Git fetch 并行数
git-fetch-with-cli = true
# 离线模式
# offline = false
# HTTP 超时（秒）
# timeout = 30

[http]
# 代理设置
# proxy = "127.0.0.1:7890"
# 低速限制（字节/秒）
# low-speed-limit = 10
# 多路复用
multiplexing = true
# SSL 证书验证
# check-revoke = true

# ============================================================================
# 别名配置
# ============================================================================

[alias]
# 常用别名
b = "build"
c = "check"
t = "test"
r = "run"
rr = "run --release"
br = "build --release"

# 清理并构建
cb = "clean && build"

# 带详细输出的测试
tv = "test -- --nocapture"

# Clippy 检查
cl = "clippy -- -D warnings"
cla = "clippy -- -A unused-variables -A dead-code"

# 格式化
fmt-check = "fmt -- --check"

# 更新所有依赖
up = "update"

# 显示依赖树
tree = "tree --depth 3"

# 快速检查（不生成二进制）
qc = "check --all-targets"

# Watch 模式（需要 cargo-watch）
w = "watch -x check -x test -x run"

# Expand 宏（需要 cargo-expand）
exp = "expand"

# ============================================================================
# 环境变量
# ============================================================================

[env]
# SQLX 相关
SQLX_OFFLINE = "true"                    # 使用离线模式（.sqlx 文件）
# DATABASE_URL = "sqlite:./dev.db"       # 数据库 URL
RUST_BACKTRACE = "1"                     # 显示回溯
RUST_LOG = "debug"                       # 日志级别

# 加速编译
CARGO_INCREMENTAL = "1"                  # 增量编译
CARGO_HTTP_MULTIPLEXING = "true"         # HTTP 多路复用

# 使用 sccache 或 ccache 缓存编译结果
# RUSTC_WRAPPER = "sccache"              # 需要安装 sccache

# ============================================================================
# Term 配置（终端输出）
# ============================================================================

[term]
# 输出颜色
color = "auto"  # auto / always / never
# 进度条
progress.when = "auto"
progress.width = 80
# 详细程度
verbose = false
quiet = false

# ============================================================================
# Registry 配置（发布包相关）
# ============================================================================

[registry]
# 发布时的默认注册表
default = "crates-io"
# 令牌（通常存储在 credentials 文件中）
# token = "your-token-here"

# ============================================================================
# Future 不稳定特性（需要 nightly）
# ============================================================================

# [unstable]
# build-std = ["std", "panic_abort"]     # 从源码构建 std
# timings = ["html", "json"]              # 编译时间分析
```

## SQLX 特定配置

对于 SQLX，你可能还需要：

```toml
[env]
# SQLX 离线模式（推荐用于 CI/CD）
SQLX_OFFLINE = "true"

# 数据库连接 URL
DATABASE_URL = "sqlite:./data/dev.db"
# 或 PostgreSQL
# DATABASE_URL = "postgres://user:pass@localhost/dbname"

# SQLX 日志
SQLX_LOG = "info"
SQLX_LOG_LEVEL = "info"
```

然后需要生成离线查询数据：

```bash
cargo sqlx prepare
```

## 推荐的开发配置组合

```toml
[build]
jobs = 8
incremental = true

[target.'cfg(all())']
rustflags = [
    "-A", "unused-variables",
    "-A", "dead-code",
    "-C", "link-arg=-fuse-ld=lld",  # macOS 需要调整路径
]

[profile.dev.package."*"]
opt-level = 1

[env]
SQLX_OFFLINE = "true"
RUST_BACKTRACE = "1"
RUST_LOG = "info"
CARGO_INCREMENTAL = "1"

[alias]
w = "watch -x check -x test"
cl = "clippy -- -A unused-variables -A dead-code"

[source.crates-io]
replace-with = "rsproxy"  # 选择你网络最快的源

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"
```

这样配置可以显著提升开发体验！
