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

#### 基本配置

```yaml
name: release-plz

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: true
```

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

### 工作流任务详解

#### 1. release-plz-pr 任务

**触发条件**：当有代码推送到 main 分支时

**安全检查**：

```yaml
if: ${{ github.repository_owner == 'YOUR_ORG' }}
```

**主要步骤**：

1. 检出代码（完整历史）
2. 安装 Rust 工具链
3. 缓存 Cargo 注册表、索引和构建产物
4. 运行 release-plz 创建发布 PR

**环境变量**：

- `GITHUB_TOKEN`: 用于创建和操作 Pull Request
- `CARGO_REGISTRY_TOKEN`: 用于发布到 crates.io（当前配置为不发布）

**详细配置**：

```yaml
release-plz-pr:
  name: Release PR
  runs-on: ubuntu-latest
  if: ${{ github.repository_owner == 'YOUR_ORG' }}
  steps:
    - name: Checkout repository
      uses: actions/checkout@v5
      with:
        fetch-depth: 0
        token: ${{ secrets.GITHUB_TOKEN }}

    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable

    - name: Cache cargo registry
      uses: actions/cache@v4
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v4
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v4
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

    - name: Run release-plz
      uses: release-plz/action@v5
      with:
        command: release-pr
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

#### 2. release-plz-release 任务

**触发条件**：当 release-plz 创建的 PR 被合并时

**触发条件详解**：

```yaml
if: ${{ github.event_name == 'pull_request' && github.event.pull_request.merged == true && startsWith(github.event.pull_request.head.ref, 'release-plz-') && github.repository_owner == 'YOUR_ORG' }}
```

**并发控制**：

```yaml
concurrency:
  group: release-plz-release-${{ github.ref }}
  cancel-in-progress: true
```

**主要步骤**：

1. 检出代码（完整历史）
2. 安装 Rust 工具链
3. 缓存 Cargo 注册表、索引和构建产物
4. 运行 release-plz 执行发布流程

**详细配置**：

```yaml
release-plz-release:
  name: Release
  runs-on: ubuntu-latest
  if: ${{ github.event_name == 'pull_request' && github.event.pull_request.merged == true && startsWith(github.event.pull_request.head.ref, 'release-plz-') && github.repository_owner == 'YOUR_ORG' }}
  concurrency:
    group: release-plz-release-${{ github.ref }}
    cancel-in-progress: true
  steps:
    - name: Checkout repository
      uses: actions/checkout@v5
      with:
        fetch-depth: 0
        token: ${{ secrets.GITHUB_TOKEN }}

    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable

    - name: Cache cargo registry
      uses: actions/cache@v4
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v4
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v4
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

    - name: Run release-plz
      id: release-plz
      uses: release-plz/action@v5
      with:
        command: release
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

#### 3. build-and-upload-binaries 任务

**触发条件**：当 release-plz-release 任务成功完成后

**依赖关系**：

```yaml
needs: release-plz-release
```

**矩阵构建策略**：

```yaml
strategy:
  matrix:
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
        artifact_name: board
        asset_name: board-linux-x86_64
      - os: windows-latest
        target: x86_64-pc-windows-msvc
        artifact_name: board.exe
        asset_name: board-windows-x86_64.exe
      - os: macos-latest
        target: x86_64-apple-darwin
        artifact_name: board
        asset_name: board-macos-x86_64
```

**主要步骤**：

1. 检出代码（完整历史）
2. 安装 Rust 工具链和目标平台
3. 缓存 Cargo 注册表、索引和构建产物
4. 构建多平台二进制文件
5. 上传二进制文件到 GitHub Release

**详细配置**：

```yaml
build-and-upload-binaries:
  name: Build and Upload Binaries
  runs-on: ${{ matrix.os }}
  needs: release-plz-release
  if: ${{ github.event_name == 'pull_request' && github.event.pull_request.merged == true && startsWith(github.event.pull_request.head.ref, 'release-plz-') && github.repository_owner == 'YOUR_ORG' }}
  strategy:
    matrix:
      include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
          artifact_name: board
          asset_name: board-linux-x86_64
        - os: windows-latest
          target: x86_64-pc-windows-msvc
          artifact_name: board.exe
          asset_name: board-windows-x86_64.exe
        - os: macos-latest
          target: x86_64-apple-darwin
          artifact_name: board
          asset_name: board-macos-x86_64
  steps:
    - name: Checkout repository
      uses: actions/checkout@v5
      with:
        fetch-depth: 0

    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}

    - name: Cache cargo registry
      uses: actions/cache@v4
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v4
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v4
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}

    - name: Build binary
      run: cargo build --release --target ${{ matrix.target }}

    - name: Upload binary to release
      uses: softprops/action-gh-release@v2
      with:
        files: target/${{ matrix.target }}/release/${{ matrix.artifact_name }}
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### release-plz 配置

项目的 release-plz 配置位于 `.release-plz.toml` 文件中，主要配置包括：

- 工作区级别的 changelog 自动更新
- 启用 GitHub 发布以支持二进制文件发布
- 启用 git 标签
- 默认不发布到 crates.io
- 启用 semver 检查
- PR 分支前缀：`release-plz-`
- PR 标签：`["release"]`
- 发布超时设置：10 分钟

### 使用方法

1. **开发新功能**：正常在 main 分支上进行开发
2. **自动创建发布 PR**：当有符合触发条件的提交推送到 main 分支时，GitHub Actions
   会自动创建一个包含版本更新和 changelog 的 PR
3. **审核和合并**：审核自动生成的 PR，确认无误后合并
4. **自动发布**：合并 PR 后，GitHub Actions 会自动执行发布流程，创建 git 标签和
   GitHub Release

### 版本触发规则

release-plz 会根据以下提交类型触发版本更新：

```
^(feat|fix|perf|refactor|docs|style|test|chore|build|ci):
```

### 缓存策略

工作流使用三层缓存优化构建性能：

1. **Cargo 注册表缓存**：缓存下载的 crate 包
2. **Cargo git 依赖缓存**：缓存 git 依赖
3. **构建产物缓存**：缓存编译结果

### 支持的平台和文件

**支持的平台**：

- Linux (x86_64-unknown-linux-gnu)
- Windows (x86_64-pc-windows-msvc)
- macOS (x86_64-apple-darwin)

**二进制文件命名**：

- `board-linux-x86_64`
- `board-windows-x86_64.exe`
- `board-macos-x86_64`

### 安全特性

#### 仓库所有者检查

所有关键作业都包含仓库所有者检查：

```yaml
if: ${{ github.repository_owner == 'YOUR_ORG' }}
```

这确保工作流只在主仓库中运行，防止在 fork 中意外执行。

#### 权限最小化

仅授予必要的权限：

```yaml
permissions:
  contents: write
  pull-requests: write
```

#### 并发控制

- **全局并发控制**：防止多个工作流同时运行
- **发布作业并发控制**：确保发布流程的原子性
- **取消进行中的作业**：避免资源浪费

### 最新优化内容

#### 安全增强

- **仓库所有者检查**：防止在 fork 中运行
- **升级依赖**：将 `actions/checkout` 从 v4 升级到 v5
- **权限优化**：仅授予必要的权限

#### 性能优化

- **三层缓存**：注册表、索引、构建产物缓存
- **矩阵构建**：并行构建多个平台
- **条件触发**：只在必要时触发二进制构建

#### 可靠性提升

- **并发控制**：防止资源冲突
- **依赖管理**：明确的作业依赖关系
- **错误处理**：完善的错误检查和报告

### 其他工作流

#### release-binaries.yml

项目还包含一个独立的工作流文件 `release-binaries.yml`，用于在 GitHub Release
发布时构建二进制文件：

```yaml
name: Release Binaries

on:
  release:
    types: [published]
```

这个工作流支持额外的 macOS ARM64 构建：

- macOS (aarch64-apple-darwin)
- 文件命名：`board-macos-aarch64`

### Rust 目标平台问题修复记录

#### 问题描述

在 GitHub Actions 交叉编译过程中遇到了以下错误：

```
error[E0463]: can't find crate for `std`
  |
  = note: the `x86_64-apple-darwin` target may not be installed
  = help: consider downloading the target with `rustup target add x86_64-apple-darwin`
```

#### 根本原因

问题出现在以下场景：

1. **交叉编译环境**：在非 macOS 环境中尝试编译 macOS 目标平台
2. **目标平台缺失**：`dtolnay/rust-toolchain@stable` action 虽然支持 `targets`
   参数，但在某些情况下可能无法正确安装所有目标平台
3. **构建矩阵复杂性**：项目支持多个目标平台，包括：
   - `x86_64-unknown-linux-gnu` (Linux)
   - `x86_64-pc-windows-msvc` (Windows)
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS ARM64)

#### 解决方案

在两个工作流文件中都添加了明确的目标平台安装步骤：

**release-plz.yml 修复**：

```yaml
- name: Install Rust toolchain
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: ${{ matrix.target }}

- name: Add target platform
  run: rustup target add ${{ matrix.target }}
```

**release-binaries.yml 修复**：

```yaml
- name: Install Rust toolchain
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: ${{ matrix.target }}

- name: Add target platform
  run: rustup target add ${{ matrix.target }}
```

#### 修复原理

1. **双重保障**：既使用 `dtolnay/rust-toolchain` 的 `targets` 参数，又显式调用
   `rustup target add`
2. **确保安装**：`rustup target add` 命令会确保目标平台被正确安装，即使 action
   的 `targets` 参数失败
3. **兼容性**：这种方案对所有目标平台都有效，包括交叉编译场景
4. **最小影响**：只是添加了一个额外的步骤，不会影响现有的构建流程

#### 验证方法

修复后可以通过以下方式验证：

1. **本地测试**：

```bash
# 检查目标平台是否已安装
rustup target list --installed

# 手动安装目标平台
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# 测试交叉编译
cargo build --release --target x86_64-apple-darwin
```

2. **CI/CD 验证**：

- 触发 GitHub Actions 工作流
- 检查构建日志中是否出现目标平台安装步骤
- 确认所有平台的二进制文件都能成功构建

#### 最佳实践

基于这次修复，总结出的 Rust 交叉编译最佳实践：

1. **明确指定目标平台**：始终在工作流中明确指定需要的目标平台
2. **双重安装策略**：同时使用 action 的 `targets` 参数和 `rustup target add`
   命令
3. **缓存策略**：为每个目标平台使用独立的构建缓存键
4. **矩阵构建**：使用 GitHub Actions 的矩阵策略并行构建多个平台
5. **错误处理**：在构建步骤中添加适当的错误检查和日志输出

#### 相关资源

- [Rust 交叉编译官方文档](https://doc.rust-lang.org/cross-compile/)
- [rustup 目标管理](https://rust-lang.github.io/rustup/cross-compilation.html)
- [GitHub Actions 矩阵策略](https://docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs)
- [dtolnay/rust-toolchain Action](https://github.com/dtolnay/rust-toolchain)

### 注意事项

1. **仓库设置**：确保 GitHub 仓库设置中允许 Actions 创建和操作 Pull Request
2. **密钥配置**：如需发布到 crates.io，需要配置 `CARGO_REGISTRY_TOKEN` 密钥
3. **发布配置**：当前配置不自动发布到 crates.io，如需启用请修改
   `.release-plz.toml`
4. **版本兼容**：工作流使用最新的 release-plz-action v5 版本
5. **构建时间**：二进制文件构建需要额外的构建时间，请耐心等待
6. **网络环境**：多平台构建可能需要较长时间，建议在网络良好环境下进行

### 故障排除

#### 常见问题

1. **权限问题**：
   - 检查 GitHub Actions 权限设置
   - 验证仓库设置中的 Actions 权限
   - 确认 GITHUB_TOKEN 权限配置

2. **配置错误**：
   - 检查 release-plz 配置文件语法
   - 验证 Rust 项目结构是否正确
   - 确认仓库所有者设置

3. **构建失败**：
   - 检查依赖版本兼容性
   - 验证目标平台工具链安装
   - 查看详细构建日志

4. **发布问题**：
   - 确认 PR 合并状态
   - 检查触发条件是否满足
   - 验证 git 标签创建情况

#### 调试技巧

1. **查看日志**：
   - GitHub Actions 执行日志
   - 各步骤的详细输出
   - 错误信息和堆栈跟踪

2. **本地测试**：
   ```bash
   # 检查配置
   release-plz config

   # 预览 changelog
   release-plz changelog

   # 验证版本号
   release-plz verify
   ```

3. **配置验证**：
   ```bash
   # 验证配置文件语法
   toml lint .release-plz.toml

   # 检查 workspace 配置
   cargo metadata --format-version 1
   ```

### 相关链接

- [release-plz 官方文档](https://release-plz.ieni.dev/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [release-plz-action GitHub](https://github.com/release-plz/action)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)
- [Conventional Commits 规范](https://www.conventionalcommits.org/)
