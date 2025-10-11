# GitHub Actions 修复：GitHub Releases 标签缺失问题

## 问题描述

在 GitHub CI/CD 流程中，`softprops/action-gh-release@v2` 报错：
```
Error: ⚠️ GitHub Releases requires a tag
```

## 问题分析

### 根本原因
1. **release-plz 配置问题**：`.release-plz.toml` 中 `release_always = false`
2. **执行顺序问题**：`softprops/action-gh-release@v2` 需要已存在的标签才能上传二进制文件
3. **触发条件不匹配**：release-plz 只在合并发布 PR 时创建标签，但工作流在直接推送到 main 分支时执行

### 工作流分析
- `release-plz-release` job：使用 `release-plz/action@v0.5.118` 执行 release 命令
- `build-and-upload-binaries` job：依赖前者，使用 `softprops/action-gh-release@v2` 上传二进制文件
- 当没有发布 PR 时，release-plz 不会创建标签，导致后续步骤失败

## 解决方案

### 修改配置文件

**文件**：`.release-plz.toml`

**变更**：
```toml
# 修复前
release_always = false  # 只在合并发布 PR 时发布，而不是每次提交都发布

# 修复后
release_always = true  # 在每次推送到 main 分支时都发布，确保标签创建用于二进制文件上传
```

### 配置说明

关键配置项：
- `git_tag_enable = true`：启用 git 标签创建
- `git_release_enable = true`：启用 GitHub 发布
- `release_always = true`：在每次符合条件的推送时都创建发布
- `release_commits = "^(feat|fix|perf|refactor|docs|style|test|chore|build|ci):"`：定义触发发布的提交类型

## 修复效果

### 修复前的工作流
1. 推送到 main 分支
2. release-plz-release job 执行，但不创建标签（因为 release_always = false）
3. build-and-upload-binaries job 执行，但找不到标签而失败

### 修复后的工作流
1. 推送到 main 分支（包含符合 release_commits 的提交）
2. release-plz-release job 执行并创建标签和 GitHub Release
3. build-and-upload-binaries job 执行，找到标签并成功上传二进制文件

## 验证方法

1. 检查 `.release-plz.toml` 配置是否正确
2. 确认 GitHub 工作流依赖关系
3. 测试完整的发布流程

## 最佳实践

1. **版本控制**：确保所有配置变更都有明确的提交信息
2. **测试验证**：在测试分支验证修复效果后再应用到 main 分支
3. **文档更新**：及时更新相关文档，记录配置变更原因和效果
4. **监控告警**：设置 CI/CD 失败告警，及时发现问题

## 相关资源

- [release-plz 官方文档](https://release-plz.dev/docs/config)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)

## 修复日期

2025-10-11

## 修复人员

AI Assistant (Roo)
