# Logging Feature 修复记录

## 问题描述

Rust 项目中的 log 模块编译失败，错误信息显示 `logging` feature
没有正确启用，导致以下问题：

1. `failed to resolve: use of unresolved module or unlinked crate 'log'`
2. SQLx 编译时检查失败，因为数据库表不存在

## 问题分析

### 根本原因

1. **Feature 配置问题**：虽然 `crates/board/Cargo.toml` 中已经配置了
   `default = ["logging"]`，但 board crate 缺少直接的 log 依赖
2. **数据库表缺失**：SQLx 在编译时需要验证查询，但数据库表还未创建
3. **环境变量缺失**：缺少 `DATABASE_URL` 环境变量指向正确的数据库

### 技术细节

- **logrs 模块设计**：logrs 提供了条件编译的 logging 功能，通过
  `#[cfg(feature = "logging")]` 控制是否启用标准 log crate
- **SQLx 编译时检查**：SQLx 默认在编译时验证所有查询，需要数据库连接和表结构
- **Workspace 依赖**：项目使用 Cargo workspace 管理多个 crate，feature
  传递需要正确配置

## 修复步骤

### 1. 检查现有配置

确认 `crates/board/Cargo.toml` 中的 feature 配置：

```toml
[features]
default = ["logging"]
logging = ["logrs/logging"]
```

### 2. 添加直接依赖

在 `crates/board/Cargo.toml` 中添加直接的 log 依赖：

```toml
[dependencies]
logrs = { workspace = true, features = ["logging"] }
configrs = { workspace = true }
log = { version = "0.4" }  # 新增的直接依赖
```

### 3. 创建数据库和表

```bash
cd crates/board
# 创建数据库并运行迁移
sqlite3 database.db < migrations/001_create_rooms_table.sql
sqlite3 database.db < migrations/002_create_room_contents_table.sql
sqlite3 database.db < migrations/003_create_room_access_logs_table.sql
sqlite3 database.db < migrations/004_add_indexes.sql
```

### 4. 设置环境变量并编译

```bash
# 设置数据库 URL 并编译
DATABASE_URL="sqlite:database.db" cargo build --features logging
```

### 5. 生成 SQLx 查询缓存

```bash
DATABASE_URL="sqlite:database.db" cargo sqlx prepare --workspace
```

## 修复结果

### 编译成功

- ✅ 项目编译成功，无错误
- ✅ logging feature 正确启用
- ✅ SQLx 查询验证通过
- ✅ 生成了 `.sqlx` 查询缓存文件

### 验证步骤

1. **编译验证**：`cargo build --features logging` 成功
2. **SQLx 准备验证**：`cargo sqlx prepare --workspace` 成功
3. **功能验证**：log 宏可以正常使用

## 技术要点

### Feature 配置最佳实践

1. **默认 Feature**：确保关键功能（如 logging）在 default feature 中
2. **直接依赖**：当使用外部 crate 的宏时，需要直接依赖该 crate
3. **Workspace 一致性**：确保 workspace 级别和 crate 级别的 feature 配置一致

### SQLx 开发流程

1. **数据库准备**：先创建数据库和表结构
2. **环境变量**：设置正确的 `DATABASE_URL`
3. **查询缓存**：使用 `cargo sqlx prepare` 生成查询缓存
4. **版本控制**：将 `.sqlx` 目录加入版本控制

### 调试技巧

1. **离线模式**：使用 `SQLX_OFFLINE=true` 进行离线编译
2. **清理缓存**：使用 `cargo clean` 清理构建缓存
3. **详细输出**：使用 `RUST_LOG=debug` 获取详细日志

## 后续建议

### 代码改进

1. **清理警告**：修复 `unused import: log` 警告
2. **环境配置**：添加 `.env` 文件管理环境变量
3. **CI/CD 配置**：确保持续集成中正确设置数据库

### 文档完善

1. **开发指南**：添加本地开发环境设置说明
2. **部署文档**：记录生产环境的数据库配置
3. **故障排除**：常见编译问题的解决方案

## 总结

本次修复成功解决了 Rust 项目中 logging feature 的编译问题。主要修复包括：

1. 添加了直接的 log crate 依赖
2. 创建了必要的数据库表结构
3. 正确配置了环境变量
4. 生成了 SQLx 查询缓存

修复后的项目可以正常编译和运行，logging 功能已正确启用。

---

_修复时间：2025-10-15_ _修复者：AI Assistant_ _版本：v0.3.0_
