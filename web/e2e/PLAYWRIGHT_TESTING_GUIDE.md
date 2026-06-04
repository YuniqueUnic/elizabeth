# Playwright + Screenplay Guide

## 目录结构

```text
web/e2e/
├── screenplay/
│   ├── abilities/
│   ├── fixtures/
│   ├── home/
│   │   ├── screens/
│   │   └── tasks/
│   ├── room/
│   │   ├── interactions/
│   │   ├── questions/
│   │   ├── screens/
│   │   └── tasks/
│   └── support/
├── specs/
│   ├── home/
│   └── room/
├── record.sh
└── run-tests.sh
```

## 分层约束

- `specs/` 只表达场景意图和断言。
- `tasks/` 组合业务动作，例如开房间、发消息、上传文件、关闭房间。
- `interactions/` 只放原子 UI 操作。
- `questions/` 只读状态，不做点击或写入。
- `screens/` 只暴露语义化 locator。
- `abilities/` 与 `support/` 负责技术接入，例如 API、clipboard、token 注入、多
  actor 上下文。

## 运行方式

在 `web/` 目录下执行：

```bash
bun run typecheck
bun run e2e:test -- --list
bun run e2e:test
```

按主题执行：

```bash
npx playwright test e2e/specs/home
npx playwright test e2e/specs/room/files.spec.ts
npx playwright test e2e/specs/room/messaging.spec.ts
```

## 编写规则

- 优先复用 `screenplay/room/**` 与 `screenplay/home/**` 里的 task/question。
- 允许在 spec 里保留少量 Playwright 断言或 `page.route`，但不要回退到
  page-object。
- 新增 locator 先放到 `screens/`。
- 新增可复用动作先放到 `interactions/`，再由 `tasks/` 组合。
- 保持 `specs/` 文件按主题拆分，不把所有场景堆进一个超长文件。
