# GitHub Actions 工作流配置

本文档描述了 elizabeth 项目中配置的 GitHub Actions 工作流程。

## release-plz 自动化发布流程

### 概述

项目使用 release-plz 工具实现自动化版本发布流程，包含三个主要工作流任务：

1. **release-plz-pr**: 自动创建包含版本更新和 changelog 的 Pull Request
2. **release-plz-release**: 合并 PR 后自动执行发布流程
3. **build-and-upload-binaries**: 构建并上传多平台二进制文件到 GitHub Release

### 工作流文件

位置：`.github/workflows/release-plz.yml`

### 配置详情

#### 权限设置

```yaml
permissions:
  contents: write
  pull-requests: write
```

#### 并发控制

```yaml
concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: true
```

### 工作流任务

#### 1. release-plz-pr 任务

**触发条件**：当有代码推送到 main 分支时

**主要步骤**：
1. 检出代码（完整历史）
2. 安装 Rust 工具链
3. 缓存 Cargo 注册表、索引和构建产物
4. 运行 release-plz 创建发布 PR

**环境变量**：
- `GITHUB_TOKEN`: 用于创建和操作 Pull Request
- `CARGO_REGISTRY_TOKEN`: 用于发布到 crates.io（当前配置为不发布）

#### 2. release-plz-release 任务

**触发条件**：当 release-plz 创建的 PR 被合并时

**主要步骤**：
1. 检出代码（完整历史）
2. 安装 Rust 工具链
3. 缓存 Cargo 注册表、索引和构建产物
4. 运行 release-plz 执行发布流程

### release-plz 配置

项目的 release-plz 配置位于 `.release-plz.toml` 文件中，主要配置包括：

- 工作区级别的 changelog 自动更新
- 禁用 GitHub 发布（可根据需要启用）
- 启用 git 标签
- 默认不发布到 crates.io
- 启用 semver 检查
- PR 分支前缀：`release-plz-`
- PR 标签：`["release"]`

### 使用方法

1. **开发新功能**：正常在 main 分支上进行开发
2. **自动创建发布 PR**：当有符合触发条件的提交推送到 main 分支时，GitHub Actions 会自动创建一个包含版本更新和 changelog 的 PR
3. **审核和合并**：审核自动生成的 PR，确认无误后合并
4. **自动发布**：合并 PR 后，GitHub Actions 会自动执行发布流程，创建 git 标签

### 版本触发规则

release-plz 会根据以下提交类型触发版本更新：

```
^(feat|fix|perf|refactor|docs|style|test|chore|build|ci):
```

### 缓存策略

工作流使用三层缓存优化构建性能：
1. Cargo 注册表缓存
2. Cargo git 依赖缓存
3. 构建产物缓存

#### 3. build-and-upload-binaries 任务

**触发条件**：当 release-plz-release 任务成功完成后

**主要步骤**：
1. 检出代码（完整历史）
2. 安装 Rust 工具链和目标平台
3. 缓存 Cargo 注册表、索引和构建产物
4. 构建多平台二进制文件
5. 上传二进制文件到 GitHub Release

**支持的平台**：
- Linux (x86_64-unknown-linux-gnu)
- Windows (x86_64-pc-windows-msvc)
- macOS (x86_64-apple-darwin)

**二进制文件命名**：
- `board-linux-x86_64`
- `board-windows-x86_64.exe`
- `board-macos-x86_64`

### 最新优化内容

#### 安全增强

- **仓库所有者检查**：添加 `if: github.repository == github.event.repository.full_name` 防止在 fork 中运行
- **升级依赖**：将 `actions/checkout` 从 v4 升级到 v5
- **权限优化**：仅授予必要的 `contents: write` 和 `pull-requests: write` 权限

#### 并发控制

- **全局并发控制**：`group: release-plz-${{ github.ref }}`
- **发布作业并发控制**：为 `release-plz-release` 作业添加独立的并发控制
- **取消进行中的作业**：`cancel-in-progress: true`

#### 二进制文件发布

- **多平台构建**：使用矩阵策略同时构建三个平台
- **自动上传**：构建完成后自动上传到 GitHub Release
- **条件触发**：只在发布 PR 合并时触发二进制构建

### 注意事项

1. 确保 GitHub 仓库设置中允许 Actions 创建和操作 Pull Request
2. 如需发布到 crates.io，需要在 GitHub 仓库设置中配置 `CARGO_REGISTRY_TOKEN` 密钥
3. 当前配置不自动发布到 crates.io，如需启用请修改 `.release-plz.toml` 中的 `publish` 设置
4. 工作流使用最新的 release-plz-action v5 版本
5. 二进制文件构建需要额外的构建时间，请耐心等待
6. 多平台构建可能需要较长时间，建议在网络良好环境下进行

### 故障排除

如果工作流执行失败，请检查：
1. GitHub Actions 权限设置
2. 仓库设置中的 Actions 权限
3. release-plz 配置文件语法
4. Rust 项目结构是否正确

### 相关链接

- [release-plz 官方文档](https://release-plz.ieni.dev/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [release-plz-action GitHub](https://github.com/release-plz/action)