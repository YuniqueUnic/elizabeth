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

backend: /Users/unic/dev/projs/rs/elizabeth/crates/board frontend:
/Users/unic/dev/projs/rs/elizabeth/web database:
/Users/unic/dev/projs/rs/elizabeth/app.db service manage script:
/Users/unic/dev/projs/rs/elizabeth/manage_services.sh

you always stuck during exec bash cmd. recommend to use desktop-commander to do
such things which has timeout parameter can avoid such situation.

if you want to know the origin tests description and expect, please see
/Users/unic/dev/projs/rs/elizabeth/TASKs.md

## if you want to know some room UI design and UX design，please see /Users/unic/dev/projs/rs/elizabeth/功能说明.md

Project: chromium 10/30/2025, 5:44:23 PM Total time: 7.9m room-settings.spec.ts
房间设置功能测试 › 房间基础信息 › RS-001: 应该正确显示房间 URLchromium 15.8s
room-settings.spec.ts:23 房间设置功能测试 › 房间基础信息 › RS-002: 应该从 URL
中提取房间名称 chromium 15.8s room-settings.spec.ts:28 房间设置功能测试 ›
房间基础信息 › RS-003: 应该显示容量信息 chromium 15.9s room-settings.spec.ts:33
房间设置功能测试 › 过期时间设置 › RS-004: 应该可以修改过期时间 - 1 分钟 chromium
15.9s room-settings.spec.ts:40 房间设置功能测试 › 过期时间设置 › RS-005:
应该可以修改过期时间 - 1 小时 chromium 15.8s room-settings.spec.ts:49
房间设置功能测试 › 过期时间设置 › RS-006: 应该可以修改过期时间 - 1 天 chromium
15.8s room-settings.spec.ts:58 房间设置功能测试 › 过期时间设置 › RS-007:
应该可以修改过期时间 - 永不过期 chromium 15.8s room-settings.spec.ts:67
房间设置功能测试 › 房间密码设置 › RS-008: 应该可以设置房间密码 chromium 15.8s
room-settings.spec.ts:78 房间设置功能测试 › 房间密码设置 › RS-009:
应该可以清空房间密码 chromium 16.2s room-settings.spec.ts:86 房间设置功能测试 ›
房间密码设置 › RS-010: 应该支持特殊字符密码 chromium 16.0s
room-settings.spec.ts:95 房间设置功能测试 › 房间密码设置 › RS-011:
应该支持长密码 chromium 16.0s room-settings.spec.ts:103 房间设置功能测试 ›
最大查看次数设置 › RS-012: 应该可以设置最大查看次数 chromium 16.4s
room-settings.spec.ts:113 房间设置功能测试 › 最大查看次数设置 › RS-013:
应该可以增加最大查看次数 chromium 15.9s room-settings.spec.ts:120
房间设置功能测试 › 最大查看次数设置 › RS-014: 应该可以减少最大查看次数 chromium
15.9s room-settings.spec.ts:131 房间设置功能测试 › 最大查看次数设置 › RS-015:
最大查看次数应该接受小数值 chromium 16.1s room-settings.spec.ts:141
房间设置功能测试 › 设置保存 › RS-016: 应该可以保存单个设置 chromium 16.0s
room-settings.spec.ts:150 房间设置功能测试 › 设置保存 › RS-017:
应该可以保存多个设置 chromium 16.9s room-settings.spec.ts:161 房间设置功能测试 ›
设置保存 › RS-018: 应该可以多次保存设置 chromium 15.9s room-settings.spec.ts:177
房间设置功能测试 › 权限管理 › RS-019: 应该可以切换预览权限 chromium 16.0s
room-settings.spec.ts:195 房间设置功能测试 › 权限管理 › RS-020:
应该可以切换编辑权限 chromium 16.0s room-settings.spec.ts:206 房间设置功能测试 ›
权限管理 › RS-021: 应该可以切换分享权限 chromium 15.8s room-settings.spec.ts:216
房间设置功能测试 › 权限管理 › RS-022: 应该可以切换删除权限 chromium 16.3s
room-settings.spec.ts:226 房间设置功能测试 › 权限管理 › RS-023:
应该可以保存权限设置 chromium 15.8s room-settings.spec.ts:236 房间设置功能测试 ›
权限管理 › RS-024: 应该支持所有权限组合 chromium 15.8s room-settings.spec.ts:252
房间设置功能测试 › 分享功能 › RS-025: 应该显示分享按钮 chromium 15.9s
room-settings.spec.ts:271 房间设置功能测试 › 分享功能 › RS-026:
应该显示下载二维码按钮 chromium 16.0s room-settings.spec.ts:277 房间设置功能测试
› 设置表单交互 › RS-027: 输入框应该支持焦点和取消焦点 chromium 16.1s
room-settings.spec.ts:285 房间设置功能测试 › 设置表单交互 › RS-028: 应该可以使用
Tab 键在表单中导航 chromium 15.8s room-settings.spec.ts:295 房间设置功能测试 ›
设置表单交互 › RS-029: 应该可以使用 Enter 键提交表单 chromium 17.2s
room-settings.spec.ts:304
