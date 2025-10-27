# OpenAPI 文档与 Scalar UI 修复记录

## 日期

2025-10-27

## 问题描述

服务器启动后，虽然显示 Scalar 监听在
`http://127.0.0.1:4092/api/v1/scalar`，但访问该地址时遇到以下问题：

1. **Scalar UI 无法正常显示**：页面空白或无法加载
2. **OpenAPI JSON 文档不完整**：访问 `/api/v1/openapi.json`
   返回的文档只包含一个端点，缺少实际的 API 端点信息

## 根本原因分析

### 1. 路由注册方法错误

**问题所在：**

- `crates/board/src/route/room.rs` 使用了 `.route()` 方法注册路由
- `crates/board/src/route/auth.rs` 使用了 `.route()` 方法注册路由

**问题根因：**

- `.route()` 方法只是普通的 Axum 路由注册，不会自动收集处理器函数上的
  `#[utoipa::path]` 注解
- 导致这些端点的 OpenAPI 文档信息丢失

**正确方式：**

- 应该使用 `utoipa_axum` 的 `.routes(routes!(handler_name))` 宏
- 该宏会自动收集 `#[utoipa::path]` 注解并生成 OpenAPI 文档

**对比示例：**

- ❌ 错误：`.route("/api/v1/rooms/{name}", axum_post(create))`
- ✅ 正确：`.routes(routes!(crate::handlers::rooms::create))`

### 2. Content Security Policy (CSP) 限制

**问题所在：**

- Scalar UI 需要从 CDN 加载脚本和资源
- 默认的 CSP 策略过于严格，阻止了必要的外部资源加载

**需要的域名：**

- `https://cdn.jsdelivr.net` - Scalar 主脚本
- `https://fonts.scalar.com` - Scalar 字体文件

### 3. 独立的 openapi.json 端点问题

**问题所在：**

- 原有的 `openapi()` 处理器返回静态的 `ApiDoc::openapi()`
- 该文档只在编译时注册，无法反映运行时合并的完整 API 信息

**解决方案：**

- 移除独立的 `/api/v1/openapi.json` 端点
- Scalar 自动在 `/api/v1/scalar/openapi.json` 提供完整文档

## 修复方案

### 1. 修复路由注册方式

#### route/room.rs

```rust
use std::sync::Arc;
use crate::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        // 使用 routes!() 宏替代 route() 方法
        .routes(routes!(crate::handlers::rooms::create))
        .routes(routes!(crate::handlers::rooms::find))
        .routes(routes!(crate::handlers::rooms::delete))
        .routes(routes!(crate::handlers::rooms::update_permissions))
        // ... 其他端点
        .with_state(app_state)
}
```

**关键点：**

- 必须使用完整的模块路径：`crate::handlers::rooms::create`
- 不能使用别名或重新导出的函数名
- `routes!()` 宏需要直接访问带 `#[utoipa::path]` 注解的函数

#### route/auth.rs

```rust
use std::sync::Arc;
use crate::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn auth_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::refresh_token::refresh_token))
        .routes(routes!(crate::handlers::refresh_token::revoke_token))
        .routes(routes!(crate::handlers::refresh_token::cleanup_expired_tokens))
        .with_state(app_state)
}
```

### 2. 更新 Content Security Policy

#### middleware/security.rs

```rust
// Note: cdn.jsdelivr.net and fonts.scalar.com are allowed for Scalar API documentation UI
router = router
    .layer(SetResponseHeaderLayer::overriding(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; \
             style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; \
             img-src 'self' data: https:; \
             font-src 'self' data: https://cdn.jsdelivr.net https://fonts.scalar.com; \
             connect-src 'self'"
        ),
    ))
```

### 3. 简化 OpenAPI 端点

#### route/mod.rs

**移除：**

- `ApiDoc` 结构体定义
- `openapi()` 处理器函数
- 相关的导入语句

**保留：**

```rust
pub fn scalar(api: utoipa::openapi::OpenApi) -> (Scalar<utoipa::openapi::OpenApi>, String) {
    let path = format!("{}/scalar", API_PREFIX);
    (Scalar::with_url(path.clone(), api), path)
}
```

#### lib.rs - build_api_router()

```rust
fn build_api_router(app_state: Arc<AppState>, cfg: &configrs::Config) -> (String, axum::Router) {
    // 直接从各子模块获取路由和 API 文档
    let (status_router, mut api) = route::status::api_router().split_for_parts();
    let (room_router, room_api) = route::room::api_router(app_state.clone()).split_for_parts();
    let (auth_router, auth_api) = route::auth::auth_router(app_state).split_for_parts();

    // 合并路由
    let router = status_router
        .merge(room_router)
        .merge(auth_router);

    // 合并 API 文档
    api.merge(room_api);
    api.merge(auth_api);

    // 添加 Scalar UI
    let (scalar, scalar_path) = route::scalar(api);
    let router = router.merge(scalar);

    // 应用中间件
    let middleware_config = crate::middleware::from_app_config(cfg);
    let router = crate::middleware::apply(&middleware_config, router);

    (scalar_path, router)
}
```

### 4. 修复测试文件

#### tests/common/mod.rs

确保测试中也使用相同的路由构建方式：

```rust
// 创建路由
let (status_router, mut api) = board::route::status::api_router().split_for_parts();
let (room_router, room_api) =
    board::route::room::api_router(app_state.clone()).split_for_parts();
let (auth_router, auth_api) =
    board::route::auth::auth_router(app_state.clone()).split_for_parts();

api.merge(room_api);
api.merge(auth_api);

let app = status_router
    .merge(room_router)
    .merge(auth_router);
```

## 验证结果

### 1. Scalar UI 正常显示

访问 `http://127.0.0.1:4092/api/v1/scalar` 可以看到：

- ✅ 完整的 API 端点分组：
  - authentication (3 个端点)
  - status (2 个端点)
  - rooms (多个端点)
  - content (多个端点)
  - chunked-upload (4 个端点)
  - Models

- ✅ 所有资源正常加载（脚本、样式、字体）
- ✅ 交互功能正常（搜索、展开/折叠、代码示例等）

### 2. 所有测试通过

```bash
cargo test -p elizabeth-board
# 所有测试用例均通过
```

### 3. 编译检查通过

```bash
cargo check -p elizabeth-board
cargo build -p elizabeth-board --release
# 无警告和错误
```

## 关键经验总结

### utoipa_axum 最佳实践

1. **路由注册方式**
   - ✅ 使用：`.routes(routes!(handler_func))`
   - ❌ 避免：`.route(path, method(handler_func))`

2. **处理器路径**
   - 必须使用完整的模块路径
   - 不能使用 `use` 导入后的别名
   - 函数必须标注 `#[utoipa::path]`

3. **API 文档合并**
   - 使用 `.split_for_parts()` 分离路由和文档
   - 使用 `api.merge()` 合并子模块的文档
   - 确保最终传给 Scalar 的是完整文档

4. **CSP 配置**
   - 为 Scalar UI 添加必要的 CDN 域名白名单
   - 保持最小权限原则
   - 添加注释说明为什么需要这些域名

### 调试技巧

1. **查看 OpenAPI JSON**
   - Scalar 提供：`/api/v1/scalar/openapi.json`
   - 检查是否包含所有端点

2. **浏览器开发者工具**
   - Console：查看 CSP 错误
   - Network：检查资源加载状态
   - Elements：确认 Scalar 组件是否渲染

3. **编译时检查**
   - `routes!()` 宏会在编译时验证
   - 确保处理器函数可见且有正确注解

## 相关文件

- `crates/board/src/route/room.rs` - Room 路由定义
- `crates/board/src/route/auth.rs` - Auth 路由定义
- `crates/board/src/route/mod.rs` - 路由模块入口
- `crates/board/src/lib.rs` - 路由构建逻辑
- `crates/board/src/middleware/security.rs` - CSP 配置
- `crates/board/tests/common/mod.rs` - 测试辅助函数

## 参考资源

- [utoipa-axum 官方文档](https://docs.rs/utoipa-axum)
- [utoipa 官方文档](https://docs.rs/utoipa)
- [Scalar 官方文档](https://github.com/scalar/scalar)
- [Axum 路由文档](https://docs.rs/axum/latest/axum/routing/)
