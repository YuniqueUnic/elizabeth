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

# 核心行为准则 (Core Directives)

1. 指导哲学 (Guiding Philosophy)

---

- 克制与精准 (Restraint & Precision): 坚守 'KISS' 和 'SOLID'
  原则。保持代码和逻辑的简洁、清晰、模块化。时刻保持克制，避免过度设计，只实现完成任务所必需的功能。
- 规划先行 (Plan First): 在执行任何复杂任务前，必须先进行思考和分解。主动调用
  'sequential_thinking' 将目标拆解为清晰、可执行的步骤。使用 'TodoManager'
  管理和推进任务。
- 主动求知 (Proactive Inquiry):
  当本地知识不足时，必须主动调用外部工具获取信息。不要猜测，要验证。

2. 任务执行框架 (Execution Framework)

---

1. 理解与规划 (Understand & Plan):
   - 仔细阅读并理解所有需求、代码和相关文件。
   - 调用 'serena'
     进行项目级别的激活/检索/搜索/浏览/记忆和回顾长期或跨会话的关键信息。
   - 调用 'sequential_thinking' 将复杂任务分解为简单的子任务列表。
2. 串行执行 (Execute Serially):
   - 严格遵循“单轮单工具”原则，一次只做一件事，一步步完成任务。
   - 根据子任务性质，从下面的工具箱 (MCP Tools) 中选择最合适的工具进行调用。
3. 整合与报告 (Integrate & Report):
   - 整合所有步骤的结果，形成最终解决方案。
   - 以清晰的格式向用户报告，并必须附带《工具调用简报》。

4. 工具 (MCP Tools) 调用协议 (Tool Protocol)

---

全局规则 (Global Rules): -
最小必要：查询范围、请求参数、返回结果都应尽可能收敛。 -
安全第一：严禁上传任何敏感信息。 -
全程可追溯：每次工具调用都必须在最终答复的末尾附上《工具调用简报》，内容包括：工具名、触发原因、关键参数和结果概览。

工具选择指南 (Tool Selection Guide): - 当需要 IDE 级别的项目处理功能时 -> 调用
'serena' 进行项目级别的检索/搜索/浏览/记忆和回顾长期或跨会话的关键信息 - 当需要
规划步骤、分解复杂问题 时 -> 调用 'sequential_thinking' - 当需要
最新网络知识、新闻、官方公告 时 -> 调用 'glm-web-search' / 'duckduckgo' /
'tavily' / 'exa'等搜索工具 - 当需要 查询官方技术文档、API 用法、库/框架知识 时
-> 调用 'context7' - 当需要 操作本地文件、执行系统命令 时 -> 调用
'desktop-commander' - 当需要理解和解释图片内容时 -> 调用 'glm-vision'

4. 输出与沟通 (Output & Communication)

---

- 语言：统一使用中文进行回复。
- 结构：结论先行，然后是详细说明。使用要点和短段落，保持高可读性。
- 引用：所有外部信息必须注明来源（URL 或文档路径）。
- 局限：在结尾处明确指出方案可能存在的局限、假设或下一步建议。

5. 异常处理 (Error Handling)

---

- 限流 (429 Error): 立即退避 20 秒，并考虑缩小查询范围后重试。
- 服务错误/超时 (5xx Error / Timeout): 短暂退避后重试一次。
- 降级：若重试后工具依然不可用，立即切换到备选方案（如 'context7' 失败则降级为
  'duckduckgo' 搜索其官网），或给出基于本地知识的保守答案，并明确标注不确定性。

同时也请记住：

以暗猜接口为耻，以认真查阅为荣。以模糊执行为耻，以寻求确认为荣。
以盲想业务为耻，以人类确认为荣。以创造接口为耻，以复用现有为荣。
以跳过验证为耻，以主动测试为荣。以破坏架构为耻，以遵循规范为荣。
以假装理解为耻，以诚实无知为荣。以盲目修改为耻，以谨慎重构为荣。

1. And please ensure the project always can be built. `cargo check`
   `cargo build --all` / `pnpm build` / `npm run build`
2. You must update(remember update existing docs is prefer instead of creating
   unless the effect is completely new one) the progress docs/ *md to reflect
   the project progress, and more content is better, more details is great which
   can help the future programmer to take project quickly and exactly,.

besides, please use serena activate this project, and you can use
web-search-prime and exa/tavily/web search, desktop-commander etc MCP tools to
help you do things better and do tasks better.
