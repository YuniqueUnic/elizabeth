# API 400 Bad Request 错误修复报告

## 问题诊断与解决总结

### 问题描述

用户在禁用房间的"分享"权限后，所有消息相关的 API 调用都返回 400 Bad Request
错误，导致前端无法加载消息、发送消息等操作。

错误堆栈：

```
Console APIError: Bad Request
lib/utils/api.ts (367:17) @ request
api/messageService.ts (67:20) @ getMessages
components/layout/middle-column.tsx (65:35) @ fetchMessages
```

### 根本原因

系统在禁用分享权限时生成的私有 slug 格式为 `{room-name}_{UUID}`，总长度可能超过
50 个字符。

后端 `content.rs` 处理程序中有 6 个位置使用了
`RoomNameValidator::validate(&name)` 进行验证，该方法限制房间标识符长度为 3-50
个字符。当传入长 slug 时，验证失败，导致 400 错误。

### 修复方案

**文件修改**: `crates/board/src/handlers/content.rs`

**改动**: 将所有使用了错误验证器的位置从 `RoomNameValidator::validate()` 改为
`RoomNameValidator::validate_identifier()`。

| 函数名               | 行号 | 说明             |
| -------------------- | ---- | ---------------- |
| `list_room_contents` | 133  | 获取房间内容列表 |
| `prepare_upload`     | 181  | 准备上传文件     |
| `upload_content`     | 294  | 上传内容         |
| `delete_content`     | 552  | 删除内容         |
| `download_content`   | 640  | 下载内容         |
| `update_content`     | 770  | 更新内容         |

### 验证方法说明

- **`validate()`**: 允许 3-50 字符的房间标识符
- **`validate_identifier()`**: 允许 3-150 字符的房间标识符（支持名称和 slug）

## 修复效果验证

### 测试场景

1. ✅ **创建房间**: 房间创建成功，获取消息 API 调用成功
2. ✅ **禁用分享权限**: 系统生成私有 slug（60+ 字符）
3. ✅ **页面重定向**: URL 自动更新到新 slug，无 API 错误
4. ✅ **获取消息**: API 成功返回消息列表（修复前返回 400 错误）
5. ✅ **发送消息**: 消息成功上传和保存（修复前无法调用 API）
6. ✅ **消息管理**: 编辑、删除等操作全部正常

### 测试结果

```
修复前: ❌ Console APIError: Bad Request
修复后: ✅ "共 1 条消息" - 消息成功显示

修复前: ❌ 无法发送消息（API 调用失败）
修复后: ✅ 消息 "测试消息 - 验证长 slug 修复" 成功发送
```

## 相关修复提交

| 提交 SHA  | 说明                                    |
| --------- | --------------------------------------- |
| `1e056c0` | 修复 token 验证器支持长 slug            |
| `e14f9d9` | 修复 content handlers 验证器支持长 slug |

## 代码质量

- ✅ 编译检查：通过
- ✅ Rustfmt 格式检查：通过
- ✅ Clippy 代码分析：通过
- ✅ Typos 拼写检查：通过
- ✅ 最小化改动：仅修改 6 行代码

## 总结

通过修改后端验证器，使其支持长房间标识符（包含 UUID 的
slug），成功解决了禁用分享权限后的 400 Bad Request
错误问题。系统现在能够正确处理所有长 slug 的 API 调用。

---

**修复状态**: ✅ COMPLETED **测试状态**: ✅ VERIFIED **生产就绪**: ✅ YES
