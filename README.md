# Elizabeth

Elizabeth 是一个基于 Rust
的文件分享和协作平台项目，旨在提供简单、安全、高效的文件共享解决方案。

## 项目概述

Elizabeth
项目致力于构建一个现代化的文件分享系统，支持多种文件类型、实时协作和高级安全特性。项目采用模块化设计，使用
Rust 语言确保高性能和内存安全。

### 核心特性

- 🚀 **高性能**: 基于 Rust 的高性能文件处理
- 🔒 **安全性**: 内存安全和数据加密
- 📁 **多格式支持**: 文本、图片、代码片段等多种文件类型
- 🌐 **Web 界面**: 现代化的用户界面
- ☁️ **云存储**: 集成 Cloudflare R2 等 S3 兼容存储

## 项目结构

```
elizabeth/
├── crates/
│   └── board/           # 核心板块功能
├── docs/
│   ├── research.md      # 研究和设计文档
│   ├── release-plz.md   # 发布系统文档
│   └── github-actions.md # CI/CD 文档
├── .github/
│   └── workflows/
│       └── release-plz.yml # 自动发布工作流
├── .release-plz.toml    # release-plz 配置
├── CHANGELOG.md         # 变更日志
├── Cargo.toml          # 项目配置
└── README.md           # 项目说明
```

## 快速开始

### 环境要求

- Rust 1.90
- Git

### 安装和构建

1. **克隆仓库**
   ```bash
   git clone https://github.com/your-username/elizabeth.git
   cd elizabeth
   ```

2. **构建项目**
   ```bash
   cargo build --release
   ```

3. **运行项目**
   ```bash
   cargo run
   ```

### 开发环境设置

1. **安装开发依赖**
   ```bash
   cargo install --dev release-plz
   cargo install --dev git-cliff
   cargo install --dev cargo-semver-checks
   ```

2. **运行测试**
   ```bash
   cargo test
   ```

3. **检查代码格式**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

## 发布系统

Elizabeth 项目使用 [release-plz](https://release-plz.ieni.dev/)
实现自动化版本发布流程。该系统能够：

- 根据 Conventional Commits 自动确定版本号
- 自动生成和维护 changelog
- 创建 GitHub Release
- 与 GitHub Actions 无缝集成

### 发布流程

1. **日常开发**: 在功能分支上进行开发，使用 Conventional Commits 格式提交
2. **合并代码**: 将功能分支合并到 main 分支
3. **自动创建发布 PR**: GitHub Actions 自动创建包含版本更新和 changelog 的 PR
4. **审核发布**: 审核自动生成的 PR，确认无误后合并
5. **自动发布**: 合并 PR 后自动执行发布流程，创建 git 标签

### Conventional Commits 规范

项目遵循 Conventional Commits 规范，支持的提交类型包括：

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

#### 提交示例

```bash
# 新功能
git commit -m "feat(auth): add user authentication"

# 修复 bug
git commit -m "fix(login): resolve token expiration issue"

# 破坏性更改
git commit -m "feat(api)!: change user endpoint response format"
```

详细的发布系统配置和使用方法请参考
[`docs/release-plz.md`](./docs/release-plz.md)。

## 开发指南

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 遵循 Rust 官方编码规范
- 编写单元测试和集成测试

### 分支策略

- `main`: 主分支，保持稳定状态
- `feature/*`: 功能分支，用于开发新功能
- `fix/*`: 修复分支，用于修复 bug
- `release-plz-*`: 发布分支，由 release-plz 自动创建

### 提交流程

1. 从 main 分支创建功能分支
2. 在功能分支上进行开发和测试
3. 提交代码，使用 Conventional Commits 格式
4. 创建 Pull Request 到 main 分支
5. 代码审查通过后合并
6. 等待自动创建发布 PR

## 文档

- [`docs/research.md`](./docs/research.md) - 研究和设计文档
- [`docs/release-plz.md`](./docs/release-plz.md) - 发布系统详细文档
- [`docs/github-actions.md`](./docs/github-actions.md) - GitHub Actions 配置文档
- [`CHANGELOG.md`](./CHANGELOG.md) - 项目变更日志

## 贡献指南

我们欢迎所有形式的贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 贡献类型

- 🐛 Bug 修复
- ✨ 新功能开发
- 📝 文档改进
- 🎨 代码优化和重构
- ⚡ 性能优化
- 🧪 测试覆盖

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 联系方式

- 项目主页：https://github.com/your-username/elizabeth
- 问题反馈：https://github.com/your-username/elizabeth/issues
- 讨论区：https://github.com/your-username/elizabeth/discussions

## 致谢

感谢所有为 Elizabeth 项目做出贡献的开发者和社区成员！

### 主要依赖

- [release-plz](https://release-plz.ieni.dev/) - 自动化发布工具
- [git-cliff](https://github.com/orhun/git-cliff) - Changelog 生成工具
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) -
  语义化版本检查

### 相关项目

- [microbin](https://github.com/szabodanika/microbin) - 灵感来源之一
- [cloudflare-drop](https://github.com/oustn/cloudflare-drop) - 参考项目

---

**Elizabeth** - 让文件分享变得简单而强大 🚀

最后更新：2025-10-11
