# Elizabeth 文件分享与协作平台

请使用 /Users/unic/dev/projs/rs/elizabeth/manage_services.sh
来控制前端和后端的服务。

> elizabeth 后端为 elizabeth-board:
> /Users/unic/dev/projs/rs/elizabeth/crates/board
>
> - 当前 elizabeth 的后端项目进度以及 API 设计等，需要时可以查看
>   /Users/unic/dev/projs/rs/elizabeth/docs/implementation 中的文档来了解
>   elizabeth 前端为 elizabeth-web: /Users/unic/dev/projs/rs/elizabeth/web
> - 当前 elizabeth 的前端项目进度以及 UI 设计，API 设计等，需要查看
>   /Users/unic/dev/projs/rs/elizabeth/web/docs/FRONTEND_DOCUMENTATION.md 来了解

## 1. 项目概览与核心目标

**Elizabeth 是什么？** Elizabeth 是一个基于 **Rust**
技术栈的现代化文件分享与协作平台。

**项目的核心目标是什么？** 项目的核心理念是 **“以房间为中心”
(Room-centric)**，而不是传统的“以用户为中心”。它旨在提供一个**安全、临时、可控**的文件共享服务。

**我们为什么这么做？（核心价值）**
我们选择这条路径是为了实现以下几个关键价值点：

- **无用户系统
  (User-less)**：摒弃传统的用户注册/登录流程。这能极大降低新用户的使用门槛。
- **房间即身份 (Room as
  Identity)**：用户通过进入一个“房间”来完成身份验证，这简化了权限管理逻辑。
- **安全优先 (Security First)**：提供全链路加密支持，包括传输安全 (TLS)
  和存储安全（规划中）。
- **临时性与可控性 (Temporality &
  Control)**：所有共享都发生在“房间”内，而房间本身是临时的，可以被精细控制。这包括设置：
  - 房间过期时间
  - 房间最大进入次数
  - 房间文件大小限制

## 核心特性

- **无用户系统**: 无需注册登录，通过房间进行身份验证
- **房间控制**: 灵活的房间设置（过期时间、密码保护、访问次数限制）
- **实时协作**: 支持 Markdown 的聊天系统，可编辑历史消息
- **文件管理**: 拖拽上传、批量下载、文件预览
- **主题切换**: 支持暗色/亮色/跟随系统三种主题模式
- **响应式设计**: 适配各种屏幕尺寸

# 核心行为准则（AGENTS.md）

目的：为 AI coding agent 提供“可执行的工程约束 +
工作协议”，避免口头约定反复解释。
原则：**简洁、可验证、可追溯**（命令优先、边界明确、输出可检查）。

> 一定要多使用 ace-tool 工具，多使用 ace-tool 工具，多使用 ace-tool
> 工具来检索和了解代码，从而更好的理解代码。
> 如果你能完美完成所有审查任务，我会给你 200 美元小费！

## 0. 范围与优先级

- 当同一仓库存在多份指令文件时，以**离当前工作目录最近**的为准；子目录会覆盖上级目录。
- 指令优先级：用户本轮需求 > 就近的 `AGENTS.override.md` > 就近的 `AGENTS.md` >
  `$CODEX_HOME/AGENTS.md`（如果存在）。

## 1. 必读（Skills）

- 写测试前：阅读并遵循 `rust-testing`（`.claude/skills/rust-testing/SKILL.md`）
- Rust 工程化开发：阅读并遵循 `rust-dev`（`.claude/skills/rust-dev/SKILL.md`）

### 1.1 禁止事项（Hard No）

- 禁止为了“加快/精简”而跳过必要的分析、验证与门禁
- 禁止以“时间限制”为理由降低质量或省略步骤
- 禁止用“让我分批处理/先做一部分”作为偷懒捷径（除非用户明确要求分批）

## 2. 核心行为准则（Core Directives）

### 2.1 禁止列表（Non-negotiable）

- 禁止“加快/精简”决策（不做草率结论）。
- 禁止“时间限制”考量（不以赶进度为理由降低质量）。
- 禁止“让我分批处理”的捷径（不要用拆批来逃避完整交付；需要拆解可以，但必须持续推进直至完成）。

### 2.2 工作哲学（Guiding Philosophy）

- 规划先行：复杂任务必须先拆解步骤，再串行执行与验证。仔细思考，可使用 ace-tool
  工具来检索和了解代码，从而更好的理解代码，拆分出任务到 tasks.csv,
  仔细思考，拆分出任务到 tasks.csv,
- 主动求知：不确定就查证（优先 web search / exa 上网查询，使用 ace-tool / rg
  来检索本地代码）；不要猜测。
- 不造轮子：优先调研成熟三方库并选型，避免自研重复基础设施。
- 稳定推进：分析并且拆分 `tasks` 至 `tasks.csv`,
  不断推进任务，形成稳定高效工作流
- 允许大改：当前未上生产，可进行大范围重构；**不需要为历史兼容背技术债**。
- KISS / DRY / SOLID：函数化、模块化、组件化、可组合；避免无效冗余与过度抽象。
- 把“功能”做成可插拔模块（插件/组件），能按需组合、可测试、边界清晰；避免在业务代码里到处散落横切逻辑（鉴权/hook/限流/追踪/缓存/注入上下文/分类/记录等）。
- GUI lib 相关参考可以阅读 /Users/unic/dev/projs/rs/syzygy/gui-demos
  中的源代码了解和参考设计：组件化界面 ui 设计，参考前端的 component, widget, ui
  文件夹这样的设计理念来完成符合本项目设计理念的 components, ui.
  从而构建可以定制化复用的组件库。为后续开发简化铺路。

记得分层设计，保持可组合性。分层设计，保持可组合性。分层设计，保持可组合性。
能拆分出来作为 crates 的就拆分出来。

- **克制与精准**：坚持 KISS / DRY / SOLID /
  LSP；保持函数化、模块化、可组合；避免无效冗余与过度设计。
- **先理解再修改**：先读需求/代码/文档/相关历史，明确现状与目标；未上生产阶段允许并鼓励在不破坏行为前提下做减债式重构。
- **不猜测，要验证**：本地检查/运行验证优先；需要外部信息时优先查官方文档与权威来源。

## 3. 执行框架（Execution Framework）

标准工作流（必须遵守）

1. 理解与规划

- 仔细阅读需求/代码/相关文档，确认现状与进度。
- 使用 **@sequential-thinking** 拆分步骤；用 **TodoManager** 推进。
- 分析并且拆分 `tasks` 至 `tasks.csv`, 不断推进任务，形成稳定高效工作流

2. 串行执行（一次只做一件事）

- 严格按步骤推进；每步结束都要能解释“做了什么、为什么、如何验证”。

3. **验证门禁**：

- 按项目类型运行 check/test/lint/build；修复所有 errors 与 warnings。

4. 整合与报告

- 输出“结论先行 + 要点短段落”。
- 必须附《工具调用简报》（见第 6 节）。
- 在完成前自检并跑质量门禁（第 1 节）。

## 5. 工具使用协议（最小必要 + 可追溯）

- **最小必要**：查询范围、请求参数、返回结果尽可能收敛。
- **安全第一**：严禁上传/回显敏感信息（Token、密钥、私有链接、内部数据等）。
- **工具选择**：
  - 规划/拆解 → `Sequential Thinking`
  - 最新信息/官方公告/外部文档 → web search / Tavily / Exa（优先官方来源）
  - 本地文件/执行命令/构建测试 → desktop-commander / shell
  - 长期记忆与跨会话要点 → memory
- **异常处理**：
  - 429：退避 20 秒；缩小查询范围后重试。
  - 5xx/超时：短暂退避后重试 1 次。
  - 仍失败：切换备选工具或给出保守方案，并明确标注不确定性。

## 6. 测试策略（按 skills/rust-testing，强制）

- **不要 inline tests**：测试集中放到
  `src/tests/**`，并与真实源码按镜像关系组织。
- 增加单元测试 + 集成测试，覆盖边缘条件与特殊情况。
- gui 的话，添加 gui-test 来预先检查和确保 gui 功能正常
- 依赖外部资源（DB/HTTP/服务）必须解耦：
  - mock：`mockall`
  - property/fake：`proptest`
  - http-mock：`wiremock` / `mockito`
- 需要 e2e 时：用 mock/fake 让测试可重复、可离线、可稳定。

## 7. 质量门禁（默认要求）

- Rust：`cargo fmt --all`、`cargo check --workspace --all-features`、`cargo test --workspace --all-features`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`
- JS/TS：`pnpm build` / `tsc` / `vite build`（按仓库实际脚本）
- 其它：优先使用仓库已有的 lint/test/build 命令；必须修复 warnings

## 8. 输出与沟通规范

- 语言：中文
- 结构：结论先行 → 要点 → 必要细节
- 引用：所有外部信息必须注明来源（URL 或文档路径）
- 局限：结尾明确指出假设、局限与下一步建议
- 《工具调用简报》：工具名 / 触发原因 / 关键参数 / 结果概览

## 9. 异常处理（Error Handling）

- 429 限流：立即退避 20 秒；缩小查询范围后重试。
- 5xx/Timeout：短暂退避后重试一次。
- 降级：工具不可用则切换备选（web search →
  tavily/exa）；否则给出保守答案并标注不确定性。
