
## MVP

- 启动 web-server
- RESTFUL API
    - 示例网站：www.unicshare.com
    - GET www.unicshare.com/\[name\] 如果存在该 name 的 room 则加载
        - 如果 room - locked by password, 需要用户输入 password
        - 进入 room
    - GET www.unicshare.com/\[name\] 如果不存在该 name 的 room 则新建该 room, 但是该 room 暂时只存在于本地 (该怎么做呢)
        - 用户可以为该 share room 设置 password, expire-time, times-of-looking, 添加文本 (配置/自动检测 language-syntax/highlight-syntax), 添加图片 (支持 wasm 客户端压缩后再上传，限制总图片大小 50m，直接像相册一样提供瀑布流预览/也支持选中两个对比), 添加文件 (限制总文件大小 100m?).
        - 用户添加完毕之后，点击保存按钮，POST 用户添加的内容到该 room 中。
    - 因此 POST 这是具有创建/删除 room 的功能，还具有上传文本/文件的功能

## GitHub Actions 修复 (2025-10-11)

### ✅ 已完成：GitHub Releases 标签错误修复

**问题描述**：
- 尽管在 `.release-plz.toml` 中设置了 `release_always = true`，但 GitHub Actions 仍然报告 `⚠️ GitHub Releases requires a tag` 错误

**根本原因**：
- 竞态条件：`build-and-upload-binaries` job 在 release-plz 完成 tag 创建后立即开始，但 tag 可能还没有在 GitHub API 中完全可用
- 缺少 tag 存在性检查：没有验证 tag 是否真的存在就开始上传二进制文件

**修复方案**：
- 在 `build-and-upload-binaries` job 中添加了 "Wait for release tag" 步骤
- 实现重试机制（最多 5 次，每次间隔 30 秒）
- 添加 "Verify release exists" 步骤验证 release 存在性
- 明确指定 tag_name 参数给 `softprops/action-gh-release`
- 添加详细的调试信息和错误处理

**修改文件**：
- `.github/workflows/release-plz.yml` - 添加了 tag 等待和验证机制
- `docs/github-actions-tag-fix.md` - 创建了详细的修复报告

**效果**：
- 消除了竞态条件问题
- 提高了工作流的可靠性
- 增强了错误诊断能力

## 其它

- 给 configrs 添加上 log dep, 然后在合适的地方添加上对应的 log