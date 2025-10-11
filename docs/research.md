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
