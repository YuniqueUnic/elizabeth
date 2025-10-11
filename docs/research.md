https://github.com/oustn/cloudflare-drop?tab=readme-ov-file
https://github.com/99percentpeople/weblink p2p WebRTC
https://github.com/szabodanika/microbin/tree/master 很接近我想要的了，但是 license 和 url 问题

https://github.com/blenderskool/blaze p2p file sharing

## Release-plz 自动化发布系统

### 概述

项目集成了 [release-plz](https://release-plz.ieni.dev/) 工具，实现自动化版本发布流程。release-plz 是一个专为 Rust 项目设计的发布自动化工具，支持 Conventional Commits 规范，能够自动生成 changelog、管理版本号，并与 GitHub Actions 无缝集成。

### 实施目标

- 自动化版本发布流程，减少手动操作错误
- 遵循 Conventional Commits 规范，提高提交质量
- 自动生成和维护 changelog
- 支持 GitHub Actions 集成，实现 CI/CD 流程
- 提供灵活的配置选项，适应不同发布需求

### 核心功能

1. **自动版本管理**: 根据提交类型自动确定版本号（主版本、次版本、补丁版本）
2. **Changelog 生成**: 基于提交历史自动生成格式化的变更日志
3. **GitHub 集成**: 自动创建发布 PR 和 GitHub Release
4. **多包支持**: 支持 workspace 中的多个包统一版本管理
5. **语义化版本检查**: 确保版本号符合语义化版本规范

### 配置文件

- `.release-plz.toml`: release-plz 主配置文件
- `.github/workflows/release-plz.yml`: GitHub Actions 工作流文件

### 支持的提交类型

- `feat`: 新功能
- `fix`: 修复 bug
- `perf`: 性能优化
- `refactor`: 代码重构
- `docs`: 文档更新
- `style`: 代码格式调整
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动
- `build`: 构建系统或依赖变更
- `ci`: CI 配置文件和脚本的变更

详细配置和使用方法请参考 [`docs/release-plz.md`](./release-plz.md)。

## Release-plz 配置修复记录 (2025-10-11)

### 问题描述

在项目初始配置阶段，release-plz 自动化发布系统遇到了多个关键问题，影响了系统的完整性和可用性：

1. **GitHub Release 功能未启用**：初始配置中 `git_release_enable = false`，导致无法自动创建 GitHub Release，用户无法获取二进制文件
2. **二进制文件发布流程缺失**：缺少自动构建和上传二进制文件到 GitHub Release 的功能，限制了项目的分发能力
3. **工作流安全问题**：GitHub Actions 工作流缺少必要的安全检查，存在在 fork 仓库中意外执行的风险
4. **并发控制不完善**：发布流程缺少适当的并发控制机制，可能导致资源冲突和发布失败
5. **Changelog 格式问题**：初始 changelog 模板过于简单，缺少标准的格式和必要的内容结构
6. **权限配置不当**：工作流权限设置过于宽泛，不符合最小权限原则

### 解决方案实施

#### 第一阶段：配置文件核心修复

**1. 启用 GitHub 发布功能**
```toml
# 修复前
git_release_enable = false  # 禁用 GitHub 发布

# 修复后
git_release_enable = true  # 启用 GitHub 发布以支持二进制文件发布
release_always = false  # 只在合并发布 PR 时发布，而不是每次提交都发布
publish_timeout = "10m"  # 设置 cargo publish 超时时间为 10 分钟
```

**2. 优化 Changelog 模板**
- 添加标准的 Keep a Changelog 格式头部
- 实现基于提交类型的分组显示
- 添加版本链接和时间戳
- 支持破坏性更改标记和作用域显示

**3. 完善配置选项**
```toml
release_always = false  # 精确控制发布时机
publish_timeout = "10m"  # 防止发布过程超时
```

#### 第二阶段：GitHub Actions 工作流安全增强

**1. 仓库所有者检查**
```yaml
# 为所有关键作业添加安全检查
if: ${{ github.repository_owner == 'YOUR_ORG' }}
```

**2. 升级依赖版本**
- 将 `actions/checkout` 从 v4 升级到 v5
- 使用最新的 release-plz-action v5
- 更新缓存策略以提高性能

**3. 权限最小化**
```yaml
permissions:
  contents: write
  pull-requests: write
```

**4. 并发控制优化**
```yaml
concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: true

# 为发布作业添加独立并发控制
concurrency:
  group: release-plz-release-${{ github.ref }}
  cancel-in-progress: true
```

#### 第三阶段：二进制文件发布流程实现

**1. 新增 build-and-upload-binaries 作业**
- 支持多平台构建：Linux、Windows、macOS
- 使用矩阵策略并行构建
- 自动上传到 GitHub Release

**2. 构建矩阵配置**
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

**3. 依赖关系管理**
```yaml
needs: release-plz-release
if: ${{ github.event_name == 'pull_request' && github.event.pull_request.merged == true && startsWith(github.event.pull_request.head.ref, 'release-plz-') && github.repository_owner == 'YOUR_ORG' }}
```

#### 第四阶段：缓存策略优化

**1. 三层缓存架构**
- Cargo 注册表缓存：缓存下载的 crate 包
- Cargo git 依赖缓存：缓存 git 依赖
- 构建产物缓存：缓存编译结果

**2. 缓存键优化**
```yaml
# 注册表缓存
key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

# git 依赖缓存
key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

# 构建产物缓存
key: ${{ runner.os }}-cargo-build-target-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}
```

### 验证结果

修复完成后，系统具备了完整的功能：

1. **自动化发布流程**：能够根据 Conventional Commits 自动确定版本号
2. **完整的 Changelog 生成**：自动生成格式规范的 changelog，包含版本链接和时间戳
3. **GitHub Release 集成**：自动创建包含二进制文件的 GitHub Release
4. **多平台支持**：支持 Linux、Windows、macOS 三个平台的二进制文件构建
5. **安全保障**：包含仓库所有者检查和适当的权限控制
6. **性能优化**：三层缓存策略显著提高构建速度
7. **可靠性提升**：完善的并发控制和错误处理机制

### 技术亮点

1. **安全性设计**：
   - 多层安全检查防止意外执行
   - 最小权限原则实施
   - 仓库所有者验证

2. **性能优化**：
   - 智能缓存策略
   - 并行构建支持
   - 增量构建优化

3. **可维护性**：
   - 标准化配置格式
   - 详细的文档说明
   - 模块化设计

4. **用户体验**：
   - 自动化程度高
   - 清晰的错误提示
   - 完整的发布流程

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

4. **性能优化**：
   - 实施多层缓存策略
   - 使用矩阵策略并行构建
   - 优化依赖管理

### 文档更新

本次修复同时更新了以下文档：

1. **[`docs/release-plz.md`](./release-plz.md)**：
   - 添加配置修复过程记录
   - 更新配置文件说明
   - 完善使用指南和故障排除

2. **[`docs/github-actions.md`](./github-actions.md)**：
   - 更新工作流详细说明
   - 添加安全特性介绍
   - 完善故障排除指南

3. **[`docs/research.md`](./research.md)**：
   - 记录完整的修复过程
   - 总结技术亮点和最佳实践

### 后续改进建议

1. **监控和告警**：
   - 添加发布流程监控
   - 实现失败告警机制

2. **自动化测试**：
   - 添加配置验证测试
   - 实现端到端发布测试

3. **文档完善**：
   - 添加视频教程
   - 创建故障排除检查清单

4. **功能扩展**：
   - 支持更多目标平台
   - 添加发布前检查流程

## MVP

1. 不能是随机的 url, 需要像 webnote
2. 支持文本/图片/文件
3. 支持代码片段渲染
4. 支持设置文件分享的密码/过期时间/最大查看次数/最大下载次数/是否允许编辑等
5. 支持图片/文件 (PDF...) 等等预览
6. 支持图片对比 (左右滑动对比)
7. 接入 cloudflare R2 等 s3.

尽量让所有的东西都在一页上显示
单一可执行文件完成部署
