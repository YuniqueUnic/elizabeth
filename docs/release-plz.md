# Release-plz 自动化发布系统

本文档详细介绍了 elizabeth 项目中集成的 release-plz
自动化发布系统的配置、使用方法和最佳实践。

## 目录

- [概述](#概述)
- [安装与配置](#安装与配置)
- [配置文件详解](#配置文件详解)
- [GitHub Actions 集成](#github-actions-集成)
- [Conventional Commits 规范](#conventional-commits-规范)
- [发布流程](#发布流程)
- [故障排除](#故障排除)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

## 概述

[release-plz](https://release-plz.ieni.dev/) 是一个专为 Rust
项目设计的发布自动化工具，它能够：

- 根据 Conventional Commits 自动确定版本号
- 自动生成和维护 changelog
- 创建 GitHub Release
- 与 GitHub Actions 无缝集成
- 支持 workspace 中的多包管理

### 主要优势

1. **自动化程度高**: 减少手动操作，降低出错概率
2. **规范化**: 强制使用 Conventional Commits 规范
3. **可视化**: 自动生成清晰的 changelog
4. **灵活性**: 支持多种配置选项
5. **集成性**: 与现有 CI/CD 流程完美集成

## 安装与配置

### 依赖安装

项目中的开发依赖已在 `Cargo.toml` 中配置：

```toml
[workspace.dependencies]
# 开发依赖
release-plz = "0.3"

# 用于 changelog 生成的依赖
git-cliff = "2.0"

# 用于 semver 检查的依赖
cargo-semver-checks = "0.44"
```

### 配置文件结构

```
.elizabeth/
├── .release-plz.toml     # 主配置文件
├── .github/
│   └── workflows/
│       └── release-plz.yml  # GitHub Actions 工作流
├── CHANGELOG.md           # 自动生成的 changelog
└── Cargo.toml            # 项目配置
```

## 配置文件详解

### .release-plz.toml

这是 release-plz 的主配置文件，包含工作区和包级别的设置：

```toml
[workspace]
# 工作区级别的默认配置
changelog_update = true  # 启用 changelog 自动更新
dependencies_update = false  # 不自动更新依赖
git_release_enable = true  # 启用 GitHub 发布以支持二进制文件发布
git_tag_enable = true  # 启用 git 标签
publish = false  # 默认不发布到 crates.io（可根据需要启用）
semver_check = true  # 启用 semver 检查
pr_branch_prefix = "release-plz-"  # PR 分支前缀
pr_labels = ["release"]  # 为发布 PR 添加标签
release_always = false  # 只在合并发布 PR 时发布，而不是每次提交都发布
publish_timeout = "10m"  # 设置 cargo publish 超时时间为 10 分钟

# 配置触发发布的提交类型
release_commits = "^(feat|fix|perf|refactor|docs|style|test|chore|build|ci):"

# 配置 changelog
[changelog]
protect_breaking_commits = true  # 始终在 changelog 中包含破坏性更改的提交
header = """
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
"""
body = """
## [{{ version }}]{%- if release_link -%}({{ release_link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
{%- if commit.scope -%}
- *({{commit.scope}})* {% if commit.breaking %}[**breaking**] {% endif %}{{ commit.message }}{%- if commit.links %} ({% for link in commit.links %}[{{link.text}}]({{link.href}}) {% endfor -%}){% endif %}
{%- else -%}
- {% if commit.breaking %}[**breaking**] {% endif %}{{ commit.message }}{% endif -%}
{% endfor -%}
{% endfor %}
"""
trim = true

# 为 board 包配置特定设置
[[package]]
name = "board"
changelog_path = "CHANGELOG.md"  # 使用根目录的 CHANGELOG
changelog_update = true
publish = false  # 暂时不发布到 crates.io
semver_check = true
version_group = "elizabeth"  # 版本组，确保 workspace 中的包使用相同版本
```

## Release-plz 配置修复过程记录

### 问题描述

在项目初始配置阶段，release-plz 自动化发布系统遇到了以下问题：

1. **GitHub Release 未启用**：初始配置中
   `git_release_enable = false`，导致无法自动创建 GitHub Release
2. **二进制文件发布缺失**：缺少自动构建和上传二进制文件到 GitHub Release 的功能
3. **工作流权限问题**：GitHub Actions 工作流缺少必要的权限配置和安全检查
4. **并发控制不完善**：发布流程缺少适当的并发控制机制
5. **Changelog 格式问题**：初始 changelog 模板过于简单，缺少必要的格式和内容

### 解决方案

#### 1. 配置文件优化

**修复前**：

```toml
git_release_enable = false  # 禁用 GitHub 发布
```

**修复后**：

```toml
git_release_enable = true  # 启用 GitHub 发布以支持二进制文件发布
release_always = false  # 只在合并发布 PR 时发布，而不是每次提交都发布
publish_timeout = "10m"  # 设置 cargo publish 超时时间为 10 分钟
```

#### 2. Changelog 模板改进

**修复前**：

```toml
[changelog]
protect_breaking_commits = true
header = ""
body = ""
trim = true
```

**修复后**：

```toml
[changelog]
protect_breaking_commits = true
header = """
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
"""
body = """
## [{{ version }}]{%- if release_link -%}({{ release_link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
{%- if commit.scope -%}
- *({{commit.scope}})* {% if commit.breaking %}[**breaking**] {% endif %}{{ commit.message }}{%- if commit.links %} ({% for link in commit.links %}[{{link.text}}]({{link.href}}) {% endfor -%}){% endif %}
{%- else -%}
- {% if commit.breaking %}[**breaking**] {% endif %}{{ commit.message }}{% endif -%}
{% endfor -%}
{% endfor %}
"""
trim = true
```

#### 3. GitHub Actions 工作流增强

**新增功能**：

- 仓库所有者检查：`if: ${{ github.repository_owner == 'YOUR_ORG' }}`
- 升级 `actions/checkout` 从 v4 到 v5
- 为 `release-plz-release` 作业添加独立的并发控制
- 新增 `build-and-upload-binaries` 作业，支持多平台二进制文件构建

**安全增强**：

```yaml
permissions:
  contents: write
  pull-requests: write

# 仓库所有者检查
if: ${{ github.repository_owner == 'YOUR_ORG' }}

# 并发控制
concurrency:
  group: release-plz-release-${{ github.ref }}
  cancel-in-progress: true
```

### 实施过程

1. **第一阶段：配置文件修复**
   - 修改 `.release-plz.toml` 中的 GitHub 发布设置
   - 优化 changelog 模板，添加标准格式
   - 添加发布超时和发布策略配置

2. **第二阶段：工作流优化**
   - 更新 GitHub Actions 工作流文件
   - 添加安全检查和权限控制
   - 实现二进制文件自动构建和上传

3. **第三阶段：测试验证**
   - 在测试环境中验证配置正确性
   - 确认所有功能按预期工作
   - 修复发现的小问题

### 验证结果

修复完成后，系统具备以下功能：

1. **自动发布流程**：能够根据 Conventional Commits 自动确定版本号
2. **Changelog 生成**：自动生成格式规范的 changelog
3. **GitHub Release**：自动创建包含二进制文件的 GitHub Release
4. **多平台支持**：支持 Linux、Windows、macOS 三个平台的二进制文件构建
5. **安全保障**：包含仓库所有者检查和适当的权限控制

### 最佳实践总结

1. **配置管理**：
   - 使用明确的配置选项，避免模糊设置
   - 为不同环境预留配置灵活性
   - 添加适当的超时和重试机制

2. **安全考虑**：
   - 始终验证仓库所有者，防止在 fork 中执行
   - 使用最小权限原则
   - 添加并发控制避免资源冲突

3. **可维护性**：
   - 使用标准化的 changelog 格式
   - 提供详细的文档和故障排除指南
   - 定期更新依赖和工具版本

#### 配置项说明

| 配置项                | 类型   | 默认值         | 说明                         |
| --------------------- | ------ | -------------- | ---------------------------- |
| `changelog_update`    | bool   | true           | 是否自动更新 changelog       |
| `dependencies_update` | bool   | false          | 是否自动更新依赖版本         |
| `git_release_enable`  | bool   | false          | 是否创建 GitHub Release      |
| `git_tag_enable`      | bool   | true           | 是否创建 git 标签            |
| `publish`             | bool   | false          | 是否发布到 crates.io         |
| `semver_check`        | bool   | true           | 是否进行语义化版本检查       |
| `pr_branch_prefix`    | string | "release-plz-" | 发布 PR 的分支前缀           |
| `pr_labels`           | array  | ["release"]    | 发布 PR 的标签               |
| `release_commits`     | string | -              | 触发发布的提交类型正则表达式 |

## GitHub Actions 集成

### 工作流文件

GitHub Actions 工作流位于
`.github/workflows/release-plz.yml`，包含三个主要任务：

#### 最新优化内容

根据 release-plz 官方文档最佳实践，工作流已进行以下优化：

1. **安全增强**：
   - 添加仓库所有者检查，防止在 fork 中运行
   - 升级 `actions/checkout` 从 v4 到 v5
   - 优化权限设置，仅授予必要的权限

2. **并发控制**：
   - 为 `release-plz-release` 作业添加并发控制
   - 防止多个发布流程同时运行

3. **二进制文件发布**：
   - 新增 `build-and-upload-binaries` 作业
   - 支持多平台构建（Linux、Windows、macOS）
   - 自动上传二进制文件到 GitHub Release
   - 只在发布 PR 合并时触发二进制构建

#### 1. release-plz-pr 任务

**触发条件**: 当有代码推送到 main 分支时

**功能**: 自动创建包含版本更新和 changelog 的 Pull Request

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

**触发条件**: 当 release-plz 创建的 PR 被合并时

**功能**: 执行实际的发布流程

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

**触发条件**: 当 release-plz-release 任务成功完成后

**功能**: 构建并上传多平台二进制文件到 GitHub Release

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

### 权限配置

```yaml
permissions:
  contents: write
  pull-requests: write
```

### 并发控制

```yaml
concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: true
```

## Conventional Commits 规范

release-plz 基于 Conventional Commits 规范来确定版本号和生成
changelog。项目中支持的提交类型包括：

### 提交类型

| 类型       | 说明                     | 版本影响         |
| ---------- | ------------------------ | ---------------- |
| `feat`     | 新功能                   | 次版本 (minor)   |
| `fix`      | 修复 bug                 | 补丁版本 (patch) |
| `perf`     | 性能优化                 | 次版本 (minor)   |
| `refactor` | 代码重构                 | 补丁版本 (patch) |
| `docs`     | 文档更新                 | 补丁版本 (patch) |
| `style`    | 代码格式调整             | 补丁版本 (patch) |
| `test`     | 测试相关                 | 补丁版本 (patch) |
| `chore`    | 构建过程或辅助工具的变动 | 补丁版本 (patch) |
| `build`    | 构建系统或依赖变更       | 补丁版本 (patch) |
| `ci`       | CI 配置文件和脚本的变更  | 补丁版本 (patch) |

### 提交格式

```bash
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

#### 示例

```bash
# 新功能
feat(auth): add user authentication

# 修复 bug
fix(login): resolve token expiration issue

# 破坏性更改
feat(api)!: change user endpoint response format

# 带作用域的提交
docs(readme): update installation instructions
```

### 破坏性更改

使用 `!` 标记破坏性更改，这将触发主版本 (major) 更新：

```bash
feat!: remove deprecated API
fix(api)!: change parameter types
```

## 发布流程

### 日常开发流程

1. **创建功能分支**
   ```bash
   git checkout -b feature/new-feature
   ```

2. **编写代码并提交**
   ```bash
   git add .
   git commit -m "feat: add new feature"
   git push origin feature/new-feature
   ```

3. **创建 Pull Request**
   - 在 GitHub 上创建 PR
   - 等待代码审查
   - 合并到 main 分支

4. **自动创建发布 PR**
   - 代码合并到 main 分支后，GitHub Actions 自动运行
   - 创建包含版本更新和 changelog 的 PR
   - PR 标题格式：`chore: release x.y.z`

5. **审核发布 PR**
   - 检查自动生成的 changelog
   - 确认版本号是否正确
   - 审核通过后合并 PR

6. **自动发布**
   - PR 合并后自动执行发布流程
   - 创建 git 标签
   - 更新 changelog
   - （可选）发布到 crates.io

### 手动触发发布

如果需要手动触发发布：

1. **本地安装 release-plz**
   ```bash
   cargo install release-plz
   ```

2. **创建发布 PR**
   ```bash
   release-plz release-pr
   ```

3. **执行发布**
   ```bash
   release-plz release
   ```

## 故障排除

### 常见问题及解决方案

#### 1. GitHub Actions 权限问题

**问题**: 工作流执行失败，提示权限不足

**解决方案**:

- 检查 GitHub 仓库设置中的 Actions 权限
- 确保 "Allow GitHub Actions to create and approve pull requests" 选项已启用
- 验证 GITHUB_TOKEN 权限配置

#### 2. 版本号冲突

**问题**: semver 检查失败

**解决方案**:

- 检查 `Cargo.toml` 中的版本号是否符合语义化版本规范
- 确认破坏性更改是否正确标记
- 手动更新版本号后重新运行

#### 3. Changelog 生成问题

**问题**: changelog 格式不正确或内容缺失

**解决方案**:

- 检查提交信息是否符合 Conventional Commits 规范
- 验证 `.release-plz.toml` 中的 changelog 配置
- 手动编辑 `CHANGELOG.md` 后重新运行

#### 4. 发布 PR 创建失败

**问题**: 无法自动创建发布 PR

**解决方案**:

- 检查 main 分支是否有新的符合触发条件的提交
- 验证 GitHub Actions 工作流配置
- 检查仓库是否有冲突的 PR

### 调试技巧

#### 1. 本地测试

```bash
# 检查配置
release-plz config

# 预览 changelog
release-plz changelog

# 验证版本号
release-plz verify
```

#### 2. 查看日志

- GitHub Actions 执行日志
- 本地运行时的详细输出
- Git 历史和标签信息

#### 3. 配置验证

```bash
# 验证配置文件语法
toml lint .release-plz.toml

# 检查 workspace 配置
cargo metadata --format-version 1
```

## 最佳实践

### 1. 提交规范

- **始终使用 Conventional Commits 格式**
- **提供清晰的提交描述**，避免模糊的表述
- **正确标记破坏性更改**，使用 `!` 符号
- **合理使用提交作用域**，提高可读性

### 2. 分支管理

- **保持 main 分支稳定**，只合并经过测试的代码
- **使用功能分支进行开发**，避免直接在 main 分支工作
- **定期合并发布 PR**，避免积累过多更改

### 3. 版本管理

- **遵循语义化版本规范**，合理确定版本号
- **及时更新 changelog**，记录重要变更
- **定期检查依赖版本**，保持项目健康

### 4. 监控和维护

- **定期检查 GitHub Actions 执行状态**
- **监控发布流程**，及时发现问题
- **备份重要配置**，防止意外丢失

## 常见问题

### Q: 如何修改版本号？

A: release-plz 会根据提交类型自动确定版本号。如果需要手动指定版本，可以：

1. 编辑 `Cargo.toml` 中的版本号
2. 提交更改：`git commit -m "chore: bump version to x.y.z"`
3. 等待自动创建发布 PR

### Q: 如何回滚发布？

A: 如果发布出现问题，可以：

1. 删除错误的 git 标签：`git tag -d v.x.y.z`
2. 推送删除：`git push origin --delete v.x.y.z`
3. 修复问题后重新发布

### Q: 如何配置发布到 crates.io？

A: 要启用 crates.io 发布：

1. 在 `.release-plz.toml` 中设置 `publish = true`
2. 在 GitHub 仓库设置中配置 `CARGO_REGISTRY_TOKEN`
3. 确保包名在 crates.io 上可用

### Q: 如何自定义 changelog 格式？

A: 可以在 `.release-plz.toml` 的 `[changelog]` 部分自定义：

```toml
[changelog]
header = "# Changelog\n\n"
body = "{{ version }} - {{ date }}\n\n{{ commits }}\n"
trim = true
```

### Q: 如何处理多包 workspace？

A: 对于 workspace 中的多个包：

1. 使用 `version_group` 确保版本同步
2. 为每个包单独配置发布设置
3. 考虑使用独立的 changelog 文件

### Q: 如何获取二进制文件？

A: 项目现在支持自动二进制文件发布：

1. **支持的平台**：
   - Linux (x86_64-unknown-linux-gnu)
   - Windows (x86_64-pc-windows-msvc)
   - macOS (x86_64-apple-darwin)

2. **文件命名**：
   - `board-linux-x86_64`
   - `board-windows-x86_64.exe`
   - `board-macos-x86_64`

3. **下载方式**：
   - 访问 GitHub Releases 页面
   - 找到对应版本的发布
   - 下载适合您平台的二进制文件

4. **自动触发**：
   - 二进制文件仅在发布 PR 合并时自动构建
   - 构建完成后会自动上传到 GitHub Release

### Q: 如何手动构建二进制文件？

A: 如果需要手动构建：

```bash
# 构建发布版本
cargo build --release

# 跨平台构建（需要安装对应目标）
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-apple-darwin
```

## 相关资源

- [release-plz 官方文档](https://release-plz.ieni.dev/)
- [Conventional Commits 规范](https://www.conventionalcommits.org/)
- [语义化版本规范](https://semver.org/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [release-plz-action GitHub](https://github.com/release-plz/action)

## 更新日志

本文档会随着项目的发展持续更新。如有问题或建议，请提交 Issue 或 Pull Request。

## GitHub Releases 标签缺失问题修复 (2025-10-11)

### 问题描述

在 GitHub CI/CD 流程中，`softprops/action-gh-release@v2` 报错：

```
Error: ⚠️ GitHub Releases requires a tag
```

### 根本原因

`.release-plz.toml` 中 `release_always = false`，导致 release-plz 只在合并发布
PR 时创建标签，但工作流在直接推送到 main 分支时执行，造成标签缺失。

### 解决方案

**修改配置**：

```toml
# 修复前
release_always = false  # 只在合并发布 PR 时发布，而不是每次提交都发布

# 修复后
release_always = true  # 在每次推送到 main 分支时都发布，确保标签创建用于二进制文件上传
```

### 修复效果

- release-plz 现在会在每次符合条件的推送到 main 分支时创建标签和 GitHub Release
- `softprops/action-gh-release@v2` 能够找到标签并成功上传二进制文件
- 完整的发布流程能够正常执行

### 相关文档

详细修复过程请参考：[GitHub Actions 修复文档](./github-actions-fix.md)

---

最后更新：2025-10-11
