# 速率限制密钥提取问题修复

## 问题描述

在启动 Elizabeth 后端服务后，所有 HTTP 请求都返回 500 错误，错误消息为：

```
HTTP/1.1 500 Internal Server Error
Unable To Extract Key!
```

## 问题分析

### 错误来源

通过搜索错误消息，发现该错误来自 `tower_governor` 库的
`GovernorError::UnableToExtractKey` 错误。

查看 `tower_governor-0.5.0/src/errors.rs`：

```rust
#[derive(Debug, Error, Clone)]
pub enum GovernorError {
    #[error("Too Many Requests! Wait for {wait_time}s")]
    TooManyRequests {
        wait_time: u64,
        headers: Option<HeaderMap>,
    },
    #[error("Unable to extract key!")]
    UnableToExtractKey,
    // ...
}
```

### 根本原因

`tower_governor`
是一个速率限制中间件，它需要从每个请求中提取一个"密钥"（通常是客户端 IP
地址）来识别和限制不同客户端的请求速率。

我们的代码存在两个问题：

1. **未指定密钥提取器**：在配置 `GovernorConfigBuilder` 时，没有指定
   `key_extractor`，导致中间件无法知道如何从请求中提取密钥。

2. **未提供 SocketAddr 信息**：服务器启动时使用了 `into_make_service()` 而不是
   `into_make_service_with_connect_info::<SocketAddr>()`，导致请求的
   `Extensions` 中没有 `SocketAddr` 信息。

## 解决方案

### 1. 添加密钥提取器

在 `crates/board/src/middleware/rate_limit.rs` 中：

```rust
// 导入 SmartIpKeyExtractor
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

// 在配置中添加 key_extractor
let governor_conf = Arc::new(
    GovernorConfigBuilder::default()
        .per_second(config.per_second)
        .burst_size(config.burst_size as u32)
        .use_headers()
        .key_extractor(SmartIpKeyExtractor)  // 添加这一行
        .finish()
        .expect("Failed to create rate limiter configuration"),
);
```

### 2. 提供 SocketAddr 信息

在 `crates/board/src/lib.rs` 中：

```rust
// 修改前
axum::serve(listener, router.into_make_service())
    .await
    .map_err(anyhow::Error::new)

// 修改后
axum::serve(
    listener,
    router.into_make_service_with_connect_info::<SocketAddr>(),
)
.await
.map_err(anyhow::Error::new)
```

## 密钥提取器说明

`tower_governor` 提供了三种内置的密钥提取器：

1. **`PeerIpKeyExtractor`**（默认）：
   - 使用请求的对等 IP 地址（peer IP address）
   - 适用于直接暴露给客户端的服务

2. **`SmartIpKeyExtractor`**（推荐）：
   - 按顺序查找常见的 IP 识别头：`x-forwarded-for`、`x-real-ip`、`forwarded`
   - 如果找不到这些头，则回退到对等 IP 地址
   - 适用于部署在反向代理（如 Nginx、Cloudflare）后面的服务

3. **`GlobalKeyExtractor`**：
   - 对所有请求使用相同的密钥
   - 适用于全局速率限制场景

我们选择了
`SmartIpKeyExtractor`，因为它能够正确处理反向代理场景，同时也能在直接访问时正常工作。

## 验证

修复后，测试创建房间：

```bash
curl -X POST "http://127.0.0.1:4092/api/v1/rooms/test-room" \
  -H "Content-Type: application/json" -v
```

成功响应：

```
HTTP/1.1 200 OK
content-type: application/json
x-ratelimit-limit: 20
x-ratelimit-remaining: 19
...

{
  "id": 1,
  "name": "test-room",
  "slug": "test-room",
  ...
}
```

可以看到：

- 请求成功返回 200 状态码
- 响应头中包含 `x-ratelimit-limit` 和
  `x-ratelimit-remaining`，说明速率限制正常工作

## 相关文档

- [tower-governor GitHub](https://github.com/benwis/tower-governor)
- [tower-governor 文档](https://docs.rs/tower_governor/)
- [Axum ConnectInfo](https://docs.rs/axum/latest/axum/extract/struct.ConnectInfo.html)

## 经验教训

1. **阅读中间件文档**：在使用第三方中间件时，务必仔细阅读其文档和示例，特别是关于配置和集成的部分。

2. **错误消息追踪**：当遇到不熟悉的错误消息时，应该：
   - 在项目代码中搜索
   - 在依赖库中搜索
   - 在 GitHub Issues 中搜索
   - 在搜索引擎中搜索

3. **理解中间件工作原理**：速率限制中间件需要识别客户端，因此必须提供客户端标识信息（如
   IP 地址）。

4. **注意部署环境**：选择合适的密钥提取器，考虑服务是否部署在反向代理后面。

## 修改文件清单

- `crates/board/src/middleware/rate_limit.rs`：添加 `SmartIpKeyExtractor`
- `crates/board/src/lib.rs`：使用
  `into_make_service_with_connect_info::<SocketAddr>()`
