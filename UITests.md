please fix those tests. I recommend you manual test firstly using browser tools
(or chrome-devtools), and during manually testing. record each operations and
items id (or path). and update the
/Users/unic/dev/projs/rs/elizabeth/web/e2e/selectors/html-selectors.ts
definition of web-items. and then correct the related playwright tests.

you can run the above operation as a loop flow till all tests are manually
passed, and all UI automation tests are passed.

if you encounter problems or issues, please check the code from
backend/frontend. and you also can use sqlite to retrieve sqlite database which
can aid you locate the issue.(if you want to reset database, please run just
reset-db)

and if you want to kill/restart/start/status backend/frontend service, please
use manage_services.sh to control

- backend: /Users/unic/dev/projs/rs/elizabeth/crates/board
- frontend: /Users/unic/dev/projs/rs/elizabeth/web
- database: /Users/unic/dev/projs/rs/elizabeth/app.db
- service manage script: /Users/unic/dev/projs/rs/elizabeth/manage_services.sh

you always stuck during exec bash cmd. recommend to use desktop-commander to do
such things which has timeout parameter can avoid such situation.

cd /Users/unic/dev/projs/rs/elizabeth/web && npx playwright test
e2e/tests/messaging.spec.ts 2>&1 | grep -A 2 "failed\|✓\|✘" | tail -50

such command will let all tasks stuck...you can run it in background and route
the output to a log file . and continue fix others... do not stuck here...

and you can look such test-result to help you Identitify issues.
/Users/unic/dev/projs/rs/elizabeth/web/test-results

if you want to know the origin tests description and expect, please see
/Users/unic/dev/projs/rs/elizabeth/TASKs.md

if you want to know some room UI design and UX design，please see
/Users/unic/dev/projs/rs/elizabeth/功能说明.md

---

messaging.spec.ts 消息系统功能测试 › 基础消息发送 › MSG-001:
应该可以发送简单文本消息 chromium 18.3s messaging.spec.ts:23 消息系统功能测试 ›
基础消息发送 › MSG-002: 应该可以发送多条消息 chromium 15.9s messaging.spec.ts:32
消息系统功能测试 › 基础消息发送 › MSG-003: 应该可以发送包含特殊字符的消息
chromium 16.6s messaging.spec.ts:44 消息系统功能测试 › 基础消息发送 › MSG-004:
应该可以发送包含 emoji 的消息 chromium 15.9s messaging.spec.ts:52
消息系统功能测试 › 基础消息发送 › MSG-005: 应该可以发送长文本消息 chromium 16.3s
messaging.spec.ts:61 消息系统功能测试 › 基础消息发送 › MSG-006:
应该可以发送换行消息 chromium 16.1s messaging.spec.ts:70 消息系统功能测试 ›
消息状态管理 › MSG-007: 发送消息后应该显示未保存标签 chromium 16.2s
messaging.spec.ts:82 消息系统功能测试 › 消息状态管理 › MSG-008:
点击保存后未保存标签应该消失 chromium 16.3s messaging.spec.ts:89
消息系统功能测试 › 消息状态管理 › MSG-009: 保存按钮在有未保存消息时应该启用
chromium 17.2s messaging.spec.ts:102 消息系统功能测试 › 消息输入框交互 ›
MSG-010: 输入框应该可以获得焦点 chromium 16.9s messaging.spec.ts:111
消息系统功能测试 › 消息输入框交互 › MSG-011: 输入框应该可以清空 chromium 16.5s
messaging.spec.ts:121 消息系统功能测试 › 消息输入框交互 › MSG-012:
应该可以选择输入框中的所有文本 chromium 15.9s messaging.spec.ts:130
消息系统功能测试 › 消息输入框交互 › MSG-013: 输入框应该可以处理粘贴操作 chromium
15.9s messaging.spec.ts:140 消息系统功能测试 › 消息输入框交互 › MSG-014:
发送按钮在有输入时应该启用 chromium 17.7s messaging.spec.ts:161 消息系统功能测试
› 消息输入框交互 › MSG-015: 发送按钮在无输入时应该禁用 chromium 16.5s
messaging.spec.ts:168 消息系统功能测试 › 消息列表交互 › MSG-016:
应该可以选择单条消息 chromium 16.1s messaging.spec.ts:177 消息系统功能测试 ›
消息列表交互 › MSG-017: 应该可以全选消息 chromium 16.8s messaging.spec.ts:192
消息系统功能测试 › 消息列表交互 › MSG-018: 应该可以反选消息 chromium 15.8s
messaging.spec.ts:204 消息系统功能测试 › 消息列表交互 › MSG-019:
消息列表应该显示消息计数 chromium 17.7s messaging.spec.ts:210 消息系统功能测试 ›
顶部栏按钮 › MSG-020: 复制按钮应该可见 chromium 16.3s messaging.spec.ts:218
消息系统功能测试 › 顶部栏按钮 › MSG-021: 下载按钮应该可见 chromium 17.3s
messaging.spec.ts:233 消息系统功能测试 › 顶部栏按钮 › MSG-022: 删除按钮应该可见
chromium 15.9s messaging.spec.ts:248 消息系统功能测试 › 顶部栏按钮 › MSG-023:
帮助按钮应该可见 chromium 16.3s messaging.spec.ts:263 消息系统功能测试 ›
顶部栏按钮 › MSG-024: 设置按钮应该可见 chromium 16.6s messaging.spec.ts:269
消息系统功能测试 › 消息流程 › MSG-025: 完整消息流程 - 发送、保存 chromium 16.1s
messaging.spec.ts:277 消息系统功能测试 › 消息流程 › MSG-026: 多消息流程 chromium
15.9s messaging.spec.ts:294 消息系统功能测试 › 边界情况 › MSG-027:
应该处理非常长的消息 chromium 15.8s messaging.spec.ts:319 消息系统功能测试 ›
边界情况 › MSG-028: 应该处理只有空格的消息 chromium 15.9s messaging.spec.ts:328
消息系统功能测试 › 边界情况 › MSG-029: 应该处理 HTML 标签内容 chromium 15.9s
messaging.spec.ts:336 消息系统功能测试 › 边界情况 › MSG-030:
应该处理连续快速发送 chromium 16.8s messaging.spec.ts:345
