# Elizabeth web - 文件分享与协作平台

> elizabeth 后端为 elizabeth-board:
> /Users/unic/dev/projs/rs/elizabeth/crates/board

当前 elizabeth 的前端项目进度以及 UI 设计，API 设计等，需要查看
/Users/unic/dev/projs/rs/elizabeth/elizabeth-web/docs/FRONTEND_DOCUMENTATION.md
来了解

Elizabeth 是一个现代化的、以房间为中心的文件分享与协作平台前端应用。

## 核心特性

- **无用户系统**: 无需注册登录，通过房间进行身份验证
- **房间控制**: 灵活的房间设置（过期时间、密码保护、访问次数限制）
- **实时协作**: 支持 Markdown 的聊天系统，可编辑历史消息
- **文件管理**: 拖拽上传、批量下载、文件预览
- **主题切换**: 支持暗色/亮色/跟随系统三种主题模式
- **响应式设计**: 适配各种屏幕尺寸

## 技术栈

- **框架**: Next.js 16 + React 19 + TypeScript
- **UI 组件**: shadcn/ui + Tailwind CSS v4
- **状态管理**: Zustand
- **数据请求**: TanStack Query (React Query)
- **Markdown**: react-markdown + remark-gfm
- **文件上传**: react-dropzone
- **日期处理**: date-fns

## 项目结构

\`\`\` elizabeth-frontend/ ├── app/ # Next.js App Router │ ├── layout.tsx #
根布局 │ ├── page.tsx # 主页面 │ └── globals.css # 全局样式 ├── components/ #
React 组件 │ ├── layout/ # 布局组件 │ │ ├── top-bar.tsx # 顶部栏 │ │ ├──
left-sidebar.tsx # 左侧边栏（房间控制） │ │ ├── middle-column.tsx#
中间栏（聊天） │ │ └── right-sidebar.tsx# 右侧边栏（文件管理） │ ├── chat/ #
聊天相关组件 │ ├── files/ # 文件管理组件 │ ├── room/ # 房间设置组件 │ └── ui/ #
基础 UI 组件（shadcn/ui） ├── api/ # API 服务层（Mock） │ ├── roomService.ts #
房间和消息 API │ ├── fileService.ts # 文件管理 API │ └── shareService.ts #
分享功能 API ├── lib/ # 工具库 │ ├── types.ts # TypeScript 类型定义 │ ├──
store.ts # Zustand 全局状态 │ ├── utils.ts # 工具函数 │ └── hooks/ # 自定义
Hooks └── package.json \`\`\`

## API 接口说明

所有 API 接口目前都是 Mock 实现，位于 `/api`
目录下。后端接入时，只需替换这些文件中的实现即可。

### 房间 API (`api/roomService.ts`)

- `getRoomDetails(roomId)` - 获取房间详情
- `updateRoomSettings(roomId, settings)` - 更新房间设置
- `getMessages(roomId)` - 获取消息列表
- `postMessage(roomId, content)` - 发送新消息
- `updateMessage(messageId, content)` - 更新消息
- `deleteMessage(messageId)` - 删除消息

### 文件 API (`api/fileService.ts`)

- `getFilesList(roomId)` - 获取文件列表
- `uploadFile(roomId, file)` - 上传文件
- `deleteFile(fileId)` - 删除文件
- `downloadFilesBatch(fileIds)` - 批量下载文件

### 分享 API (`api/shareService.ts`)

- `getQRCodeImage(roomId)` - 获取二维码图片
- `getShareLink(roomId)` - 获取分享链接

## 开发指南

### 安装依赖

\`\`\`bash npm install \`\`\`

### 启动开发服务器

\`\`\`bash npm run dev \`\`\`

访问 http://localhost:3000 查看应用。

### 构建生产版本

\`\`\`bash npm run build npm start \`\`\`

## 状态管理

使用 Zustand 管理全局状态，包括：

- 主题模式（dark/light/system）
- 发送消息快捷键设置
- 文件批量选择状态
- 侧边栏折叠状态
- 当前房间 ID

## 主要功能说明

### 1. 主题切换

三态循环切换：暗色 → 亮色 → 跟随系统 → 暗色...

### 2. 消息编辑

点击消息气泡上的"编辑"按钮，消息内容会填充到底部编辑器，修改后发送会更新原消息。

### 3. 文件批量操作

选中任意文件后，所有文件卡片会显示复选框，下载按钮显示选中数量徽章。

### 4. Markdown 支持

聊天消息支持完整的 Markdown 语法，包括：

- 标题、粗体、斜体
- 代码块和行内代码
- 列表（有序/无序）
- 链接和引用
- GFM 扩展（表格、删除线等）

## 后续接入说明

1. 替换 `/api` 目录下的 Mock 实现为真实 API 调用
2. 更新环境变量配置（如需要）
3. 调整 API 响应数据格式以匹配类型定义
4. 添加错误处理和加载状态优化
5. 实现文件上传进度显示
6. 添加 WebSocket 支持实时消息推送（可选）

## 许可证

MIT
