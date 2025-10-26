# Elizabeth 前端系统设计文档

## 目录

1. [项目概述](#项目概述)
2. [UI 设计说明](#ui-设计说明)
3. [功能说明](#功能说明)
4. [操作流程](#操作流程)
5. [架构设计](#架构设计)
6. [模块划分](#模块划分)
7. [API 接口设计](#api-接口设计)
8. [技术栈](#技术栈)

---

## 项目概述

Elizabeth 是一个基于 "房间中心"
理念的现代化文件分享与协作平台前端应用。该项目采用 React + TypeScript + Next.js
15 技术栈，提供安全、临时、可控的文件共享和实时协作功能。

### 核心特点

- **无用户系统**：无需注册登录，通过房间 ID 直接访问
- **房间即身份**：所有操作基于房间权限
- **安全优先**：支持密码保护、过期时间、访问次数限制
- **临时性与可控性**：房间和文件都是临时的，可精细控制生命周期

---

## UI 设计说明

### 整体布局结构

Elizabeth 采用经典的三栏式布局（Three-Column
Layout），整个界面分为四个主要区域：

\`\`\` ┌─────────────────────────────────────────────────────────────┐ │
顶部导航栏 (TopBar) │
├──────────┬─────────────────────────────┬────────────────────┤ │ │ │ │ │ 左侧 │
中间消息区域 │ 右侧文件 │ │ 房间控制 │ (MiddleColumn) │ 管理区域 │ │ (LeftSide│
│ (RightSidebar) │ │ bar) │ │ │ │ │ │ │ │ 可折叠 │ 消息列表 + Markdown 编辑器 │
文件列表 + 上传区 │ │ │ │ │
└──────────┴─────────────────────────────┴────────────────────┘ \`\`\`

### 1. 顶部导航栏 (TopBar)

**位置**：固定在页面顶部，高度约 64px

**组成元素**（从左到右）：

- **Logo 区域**：Elizabeth 品牌标识和房间 ID 显示
- **房间状态指示器**：显示当前房间的在线状态（绿色圆点表示活跃）
- **操作按钮组**：
  - 复制按钮（Copy）：复制选中的消息内容
  - 下载按钮（Download）：导出选中的消息为 Markdown 文件
  - 保存按钮（Save）：保存当前房间状态
  - 删除按钮（Delete）：删除选中的内容
  - 帮助按钮（Help）：打开帮助文档
  - 设置按钮（Settings）：打开设置对话框
- **主题切换器**：三态切换（暗色/亮色/跟随系统）

**视觉特点**：

- 使用半透明背景（backdrop-blur）实现毛玻璃效果
- 底部有细微的分隔线
- 按钮采用 Ghost 风格，悬停时显示背景色
- 主题切换器使用图标动画过渡

### 2. 左侧房间控制栏 (LeftSidebar)

**位置**：左侧固定，宽度 320px，可折叠

**组成部分**（从上到下）：

#### 2.1 折叠控制按钮

- 位于顶部，点击可折叠/展开整个侧边栏
- 折叠后仅显示图标按钮

#### 2.2 房间设置表单 (Room Settings)

- **过期时间选择器**：
  - 下拉选择框，提供精细的时间颗粒度
  - 选项：1 分钟、10 分钟、1 小时、12 小时、1 天、1 周、永不过期
  - 显示相对时间（如 "1 小时后过期"）
- **密码保护**：
  - 开关按钮启用/禁用密码保护
  - 启用后显示密码输入框
  - 支持显示/隐藏密码
- **最大访问次数**：
  - 数字输入框，设置房间最大进入次数
  - 范围：1-1000 次
- **保存按钮**：应用设置更改

#### 2.3 房间权限显示 (Room Permissions)

- 以标签（Badge）形式展示当前用户权限
- 权限类型：读取、编辑、分享、删除
- 不同权限使用不同颜色区分

#### 2.4 容量显示 (Room Capacity)

- 进度条显示当前使用空间 / 总空间
- 数字显示：15.5 MB / 100 MB
- 进度条颜色根据使用率变化：
  - < 70%：蓝色
  - 70-90%：黄色
  - 90%：红色

#### 2.5 分享功能 (Room Sharing)

- **二维码显示**：
  - 居中显示房间二维码
  - 支持扫码快速加入房间
- **分享链接**：
  - 显示完整的房间链接
  - 一键复制按钮
  - 复制成功后显示 Toast 提示

**视觉特点**：

- 使用 Card 组件分隔不同功能区域
- 每个区域有清晰的标题和图标
- 使用 ScrollArea 组件支持内容滚动
- 折叠动画流畅自然

### 3. 中间消息区域 (MiddleColumn)

**位置**：中间主要内容区，占据剩余宽度

**组成部分**（从上到下）：

#### 3.1 消息工具栏

- **选择控制按钮**：
  - 全选按钮：选中所有消息
  - 取消选择按钮：清除所有选择
  - 反转选择按钮：反转当前选择状态
- **选择计数器**：显示已选中的消息数量（如 "已选中 3 条消息"）

#### 3.2 消息列表区域 (Message List)

- **消息气泡 (Message Bubble)**：
  - 左侧：复选框（用于选择消息）
  - 中间：消息内容区域
    - 用户名和时间戳（顶部）
    - Markdown 渲染的消息内容
    - 支持代码高亮显示
  - 右侧：操作按钮组
    - 编辑按钮：编辑消息内容
    - 复制按钮：复制消息文本
    - 删除按钮：删除消息
- **消息样式**：
  - 选中状态：背景色高亮
  - 悬停状态：显示操作按钮
  - 代码块：使用自定义 CodeBlock 组件，支持语言标识和复制功能

#### 3.3 Markdown 编辑器 (Message Input)

- **编辑工具栏**：
  - 格式化按钮：粗体、斜体、删除线、代码、链接、列表等
  - 插入按钮：标题、引用、代码块、分隔线、表格
  - 视图切换：编辑模式、预览模式、分屏模式
  - 展开按钮：打开全屏编辑器
- **文本输入区**：
  - 多行文本框，支持 Markdown 语法
  - 实时字符计数
  - 支持键盘快捷键（Ctrl+B 加粗、Ctrl+I 斜体等）
- **发送控制**：
  - 发送按钮（右下角）
  - 支持两种发送模式：
    - Enter 发送（默认）
    - Ctrl+Enter 发送
  - 发送模式可在设置中切换

#### 3.4 全屏编辑器对话框

- **触发方式**：点击编辑器右上角的展开按钮
- **功能特点**：
  - 全屏模式，提供更大的编辑空间
  - 三种视图模式：
    - 仅编辑：只显示编辑器
    - 仅预览：只显示渲染结果
    - 分屏：左侧编辑，右侧实时预览
  - 完整的 Markdown 工具栏
  - 支持所有键盘快捷键
  - 关闭按钮和保存按钮

**视觉特点**：

- 消息列表使用虚拟滚动优化性能
- 消息气泡有圆角和阴影效果
- 代码块使用等宽字体和语法高亮
- 编辑器工具栏图标清晰易懂
- 分屏模式使用可调节的分隔条

### 4. 右侧文件管理区域 (RightSidebar)

**位置**：右侧固定，宽度 360px

**组成部分**（从上到下）：

#### 4.1 文件列表头部

- **标题**：文件管理
- **选择控制按钮**：
  - 全选按钮：选中所有文件
  - 取消选择按钮：清除所有选择
  - 反转选择按钮：反转当前选择状态
- **下载按钮**：
  - 批量下载选中的文件
  - 显示选中数量的徽章（Badge）
  - 仅在有选中文件时启用

#### 4.2 文件列表 (File List)

- **文件卡片 (File Card)**：
  - 左侧：复选框（用于选择文件）
  - 中间：文件信息
    - 缩略图（图片/视频）或文件类型图标
    - 文件名
    - 文件大小（格式化显示，如 "2.5 MB"）
    - 文件类型标签（图片、视频、PDF、文档、链接）
  - 右侧：删除按钮
- **点击行为**：
  - 点击文件卡片：打开文件预览对话框
  - 点击复选框：切换选择状态
  - 点击删除按钮：删除文件（带确认提示）

#### 4.3 文件预览对话框 (File Preview Modal)

- **触发方式**：点击文件卡片
- **预览内容**（根据文件类型）：
  - **图片**：直接显示图片预览
  - **视频**：显示视频播放器
  - **PDF**：使用 iframe 嵌入显示
  - **链接**：显示两个按钮
    - "在 iframe 中加载"：在对话框内加载网页
    - "在新标签页打开"：在新窗口打开链接
  - **其他文件**：显示下载提示
- **工具栏**：
  - 下载按钮：下载文件
  - 复制链接按钮：复制文件链接
  - 在新标签页打开按钮：在新窗口打开
  - 删除按钮：删除文件
- **关闭按钮**：右上角 X 按钮

#### 4.4 文件上传区域 (Upload Zone)

- **拖放区域**：
  - 虚线边框的拖放区域
  - 拖入文件时高亮显示
  - 显示上传图标和提示文字
- **点击上传**：
  - 点击区域打开文件选择器
  - 支持多文件选择
- **上传按钮**：
  - 主要操作按钮
  - 显示上传图标和文字

**视觉特点**：

- 文件卡片使用网格布局
- 缩略图使用圆角和边框
- 选中状态：卡片背景色高亮
- 拖放区域：虚线边框，拖入时变为实线
- 上传进度：显示进度条（如果有）

### 设计系统

#### 颜色方案

- **主色调**：蓝色系（用于主要操作按钮和链接）
- **中性色**：灰色系（用于背景、边框、次要文本）
- **语义色**：
  - 成功：绿色
  - 警告：黄色
  - 错误：红色
  - 信息：蓝色
- **主题支持**：
  - 暗色主题：深色背景，浅色文字
  - 亮色主题：浅色背景，深色文字
  - 跟随系统：根据系统设置自动切换

#### 字体系统

- **无衬线字体**：用于界面文字（Geist Sans）
- **等宽字体**：用于代码显示（Geist Mono）
- **字号层级**：
  - 标题：text-xl, text-lg
  - 正文：text-base, text-sm
  - 辅助：text-xs

#### 间距系统

- 使用 Tailwind CSS 的间距比例（4px 基准）
- 组件内边距：p-4, p-6
- 组件间距：gap-4, gap-6
- 布局间距：space-y-4, space-x-4

#### 圆角系统

- 小圆角：rounded-md (6px)
- 中圆角：rounded-lg (8px)
- 大圆角：rounded-xl (12px)
- 完全圆角：rounded-full

#### 阴影系统

- 轻微阴影：shadow-sm
- 标准阴影：shadow-md
- 强调阴影：shadow-lg

---

## 功能说明

### 1. 房间管理功能

#### 1.1 房间设置

- **过期时间管理**：
  - 支持 7 种时间颗粒度：1 分钟、10 分钟、1 小时、12 小时、1 天、1 周、永不过期
  - 实时显示剩余时间
  - 过期后房间自动失效
- **密码保护**：
  - 可选的密码保护功能
  - 密码强度验证
  - 访问时需要输入密码
- **访问次数限制**：
  - 设置房间最大进入次数
  - 达到上限后房间自动关闭
  - 实时显示剩余次数

#### 1.2 权限管理

- **权限类型**：
  - 读取（Read）：查看消息和文件
  - 编辑（Edit）：编辑消息和上传文件
  - 分享（Share）：生成分享链接和二维码
  - 删除（Delete）：删除消息和文件
- **权限显示**：
  - 以标签形式展示当前用户权限
  - 不同权限使用不同颜色

#### 1.3 容量管理

- **存储空间监控**：
  - 实时显示已使用空间和总空间
  - 进度条可视化显示使用率
  - 接近上限时警告提示
- **容量限制**：
  - 默认 100 MB 上限
  - 超出限制时禁止上传

#### 1.4 分享功能

- **二维码分享**：
  - 自动生成房间二维码
  - 支持扫码快速加入
  - 二维码包含房间 ID 和访问信息
- **链接分享**：
  - 生成唯一的房间链接
  - 一键复制链接
  - 复制成功后显示提示

### 2. 消息协作功能

#### 2.1 消息发送

- **Markdown 支持**：
  - 完整的 Markdown 语法支持
  - 实时预览功能
  - 语法高亮显示
- **富文本编辑**：
  - 工具栏快速格式化
  - 键盘快捷键支持
  - 表情符号支持（可扩展）
- **发送模式**：
  - Enter 发送模式（默认）
  - Ctrl+Enter 发送模式
  - 可在设置中切换

#### 2.2 消息管理

- **消息编辑**：
  - 点击编辑按钮进入编辑模式
  - 编辑内容自动填充到编辑器
  - 保存后更新消息内容
- **消息删除**：
  - 点击删除按钮删除消息
  - 删除前显示确认对话框
  - 删除后显示成功提示
- **消息复制**：
  - 点击复制按钮复制消息文本
  - 复制纯文本或 Markdown 格式
  - 复制成功后显示提示

#### 2.3 消息选择

- **单选**：点击消息左侧的复选框选中单条消息
- **多选**：按住 Shift 键点击可连续选择
- **全选**：点击工具栏的全选按钮选中所有消息
- **反转选择**：点击反转按钮反转当前选择状态
- **取消选择**：点击取消选择按钮清除所有选择

#### 2.4 消息导出

- **复制选中消息**：
  - 点击顶栏的复制按钮
  - 将选中的消息合并为一个文本
  - 可选择是否包含元数据（时间、消息编号）
  - 复制到剪贴板
- **导出为 Markdown**：
  - 点击顶栏的下载按钮
  - 将选中的消息导出为 .md 文件
  - 可选择是否包含元数据
  - 自动下载文件

#### 2.5 Markdown 编辑器

- **基础编辑**：
  - 多行文本输入
  - 自动保存草稿
  - 字符计数显示
- **工具栏功能**：
  - 文本格式：粗体、斜体、删除线、代码
  - 标题：H1-H6
  - 列表：有序列表、无序列表、任务列表
  - 插入：链接、图片、代码块、引用、分隔线、表格
- **全屏编辑器**：
  - 点击展开按钮打开全屏编辑器
  - 三种视图模式：编辑、预览、分屏
  - 更大的编辑空间
  - 实时预览功能

#### 2.6 代码高亮

- **语法高亮**：
  - 支持多种编程语言
  - 自动识别语言类型
  - 主题跟随系统设置
- **代码块功能**：
  - 显示语言标识
  - 一键复制代码
  - 行号显示（可选）

### 3. 文件管理功能

#### 3.1 文件上传

- **拖放上传**：
  - 拖动文件到上传区域
  - 支持多文件同时上传
  - 拖入时高亮显示
- **点击上传**：
  - 点击上传区域或按钮
  - 打开文件选择器
  - 支持多文件选择
- **上传反馈**：
  - 显示上传进度
  - 上传成功后显示提示
  - 上传失败显示错误信息

#### 3.2 文件列表

- **文件展示**：
  - 网格布局显示文件卡片
  - 显示缩略图（图片/视频）
  - 显示文件名、大小、类型
- **文件类型**：
  - 图片：显示缩略图
  - 视频：显示视频缩略图
  - PDF：显示 PDF 图标
  - 文档：显示文档图标
  - 链接：显示链接图标
- **文件排序**：
  - 按上传时间排序（默认）
  - 按文件名排序
  - 按文件大小排序

#### 3.3 文件选择

- **单选**：点击文件卡片左侧的复选框选中单个文件
- **多选**：点击多个复选框选中多个文件
- **全选**：点击工具栏的全选按钮选中所有文件
- **反转选择**：点击反转按钮反转当前选择状态
- **取消选择**：点击取消选择按钮清除所有选择

#### 3.4 文件预览

- **图片预览**：
  - 点击图片文件打开预览对话框
  - 显示原图
  - 支持缩放和旋转（可扩展）
- **视频预览**：
  - 点击视频文件打开预览对话框
  - 显示视频播放器
  - 支持播放控制
- **PDF 预览**：
  - 点击 PDF 文件打开预览对话框
  - 使用 iframe 嵌入显示
  - 支持翻页和缩放
- **链接预览**：
  - 点击链接文件打开预览对话框
  - 显示两个选项：
    - 在 iframe 中加载
    - 在新标签页打开
- **其他文件**：
  - 显示文件信息
  - 提供下载按钮

#### 3.5 文件操作

- **下载文件**：
  - 单个下载：点击预览对话框的下载按钮
  - 批量下载：选中多个文件后点击下载按钮
  - 下载为 ZIP 压缩包（批量下载）
- **删除文件**：
  - 单个删除：点击文件卡片的删除按钮
  - 批量删除：选中多个文件后点击删除按钮
  - 删除前显示确认对话框
- **复制链接**：
  - 点击预览对话框的复制链接按钮
  - 复制文件的直接访问链接
  - 复制成功后显示提示

### 4. 设置功能

#### 4.1 发送模式设置

- **Enter 发送**：按 Enter 键直接发送消息
- **Ctrl+Enter 发送**：按 Ctrl+Enter 组合键发送消息
- 设置立即生效，无需重启

#### 4.2 导出设置

- **包含元数据**：
  - 开关按钮控制
  - 元数据包括：
    - 消息时间戳
    - 消息编号
  - 影响复制和导出功能

#### 4.3 主题设置

- **三种主题模式**：
  - 暗色主题：深色背景，适合夜间使用
  - 亮色主题：浅色背景，适合白天使用
  - 跟随系统：根据系统设置自动切换
- **主题切换**：
  - 点击顶栏的主题切换器
  - 循环切换三种模式
  - 切换动画流畅自然

### 5. 通知功能

#### 5.1 Toast 通知

- **操作反馈**：
  - 成功操作：绿色提示
  - 错误操作：红色提示
  - 信息提示：蓝色提示
- **通知场景**：
  - 房间设置保存成功
  - 文件上传成功/失败
  - 文件删除成功
  - 消息发送成功/失败
  - 消息编辑成功
  - 消息删除成功
  - 复制成功
  - 下载开始
- **通知样式**：
  - 右下角弹出
  - 自动消失（3-5 秒）
  - 可手动关闭

---

## 操作流程

### 1. 进入房间流程

\`\`\` 用户访问房间链接 ↓ 检查房间是否存在 ↓ 检查房间是否过期 ↓
检查访问次数是否超限 ↓ 如果有密码保护 → 显示密码输入框 → 验证密码 ↓
进入房间主界面 ↓ 加载房间设置、消息列表、文件列表 \`\`\`

### 2. 发送消息流程

\`\`\` 用户在编辑器中输入内容 ↓ 使用工具栏格式化文本（可选） ↓
点击预览查看效果（可选） ↓ 按发送键（Enter 或 Ctrl+Enter） ↓ 调用 API 发送消息 ↓
显示发送中状态 ↓ 发送成功 → 显示成功提示 → 消息添加到列表 发送失败 →
显示错误提示 → 保留编辑内容 \`\`\`

### 3. 编辑消息流程

\`\`\` 用户点击消息的编辑按钮 ↓ 消息内容填充到编辑器 ↓ 用户修改内容 ↓
点击发送按钮 ↓ 调用 API 更新消息 ↓ 更新成功 → 显示成功提示 → 消息列表更新
更新失败 → 显示错误提示 → 保留编辑内容 \`\`\`

### 4. 导出消息流程

\`\`\` 用户选中一条或多条消息 ↓ 点击顶栏的复制或下载按钮 ↓ 系统读取选中的消息 ↓
根据设置决定是否包含元数据 ↓ 合并消息内容 ↓ 复制：复制到剪贴板 → 显示成功提示
下载：生成 Markdown 文件 → 触发下载 → 显示成功提示 \`\`\`

### 5. 上传文件流程

\`\`\` 用户拖动文件到上传区域 或 点击上传按钮选择文件 ↓ 检查文件大小是否超出限制
↓ 检查房间容量是否足够 ↓ 调用 API 上传文件 ↓ 显示上传进度 ↓ 上传成功 →
显示成功提示 → 文件添加到列表 → 更新容量显示 上传失败 → 显示错误提示 \`\`\`

### 6. 预览文件流程

\`\`\` 用户点击文件卡片 ↓ 打开文件预览对话框 ↓ 根据文件类型加载预览内容：

- 图片 → 显示图片
- 视频 → 显示视频播放器
- PDF → 使用 iframe 加载
- 链接 → 显示加载选项
- 其他 → 显示下载提示 ↓ 用户可以进行操作：
- 下载文件
- 复制链接
- 在新标签页打开
- 删除文件 ↓ 关闭对话框 \`\`\`

### 7. 批量下载文件流程

\`\`\` 用户选中一个或多个文件 ↓ 点击下载按钮 ↓ 调用 API 批量下载 ↓
服务器打包文件为 ZIP ↓ 触发浏览器下载 ↓ 显示下载成功提示 \`\`\`

### 8. 修改房间设置流程

\`\`\` 用户在左侧边栏修改设置：

- 选择过期时间
- 启用/禁用密码保护
- 设置最大访问次数 ↓ 点击保存按钮 ↓ 调用 API 更新房间设置 ↓ 更新成功 →
  显示成功提示 → 设置生效 更新失败 → 显示错误提示 → 恢复原设置 \`\`\`

### 9. 分享房间流程

\`\`\` 用户在左侧边栏查看分享区域 ↓ 选择分享方式：

- 二维码分享 → 显示二维码 → 他人扫码加入
- 链接分享 → 点击复制按钮 → 复制链接 → 发送给他人 ↓ 他人通过二维码或链接访问房间
  ↓ 进入房间流程 \`\`\`

---

## 架构设计

### 1. 技术架构

\`\`\` ┌─────────────────────────────────────────────────────────┐ │ 用户界面层
(UI Layer) │ │ ┌──────────┬──────────┬──────────┬──────────┐ │ │ │ TopBar
│LeftSide │ Middle │RightSide │ │ │ │ │ bar │ Column │ bar │ │ │
└──────────┴──────────┴──────────┴──────────┘ │
└─────────────────────────────────────────────────────────┘ ↓
┌─────────────────────────────────────────────────────────┐ │ 组件层 (Component
Layer) │ │ ┌──────────┬──────────┬──────────┬──────────┐ │ │ │ Room │ Chat │
File │ Common │ │ │ │Components│Components│Components│Components│ │ │
└──────────┴──────────┴──────────┴──────────┘ │
└─────────────────────────────────────────────────────────┘ ↓
┌─────────────────────────────────────────────────────────┐ │ 状态管理层 (State
Layer) │ │ ┌──────────────────────────────────────────┐ │ │ │ Zustand Store
(Global State) │ │ │ │ - Theme State │ │ │ │ - Settings State │ │ │ │ -
Selection State │ │ │ │ - Room State │ │ │
└──────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘ ↓
┌─────────────────────────────────────────────────────────┐ │ 数据层 (Data
Layer) │ │ ┌──────────┬──────────┬──────────┐ │ │ │ Room │ File │ Share │ │ │ │
Service │ Service │ Service │ │ │ └──────────┴──────────┴──────────┘ │
└─────────────────────────────────────────────────────────┘ ↓
┌─────────────────────────────────────────────────────────┐ │ API 层 (API Layer)
│ │ ┌──────────────────────────────────────────┐ │ │ │ RESTful API (待后端实现)
│ │ │ │ - GET /api/rooms/:id │ │ │ │ - POST /api/rooms/:id/messages │ │ │ │ -
POST /api/rooms/:id/files │ │ │ │ - ... │ │ │
└──────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘ \`\`\`

### 2. 数据流架构

\`\`\` 用户操作 (User Action) ↓ 组件事件处理 (Component Event Handler) ↓
更新本地状态 (Update Local State) - Zustand ↓ 调用 API 服务 (Call API Service) ↓
发送 HTTP 请求 (HTTP Request) - 待后端实现 ↓ 接收响应 (Receive Response) ↓
更新全局状态 (Update Global State) ↓ 触发组件重渲染 (Trigger Re-render) ↓ 更新
UI (Update UI) ↓ 显示反馈 (Show Feedback) - Toast \`\`\`

### 3. 状态管理架构

使用 **Zustand** 进行全局状态管理，状态分为以下几个部分：

#### 3.1 主题状态 (Theme State)

\`\`\`typescript { theme: "dark" | "light" | "system", setTheme: (theme) =>
void, cycleTheme: () => void } \`\`\`

#### 3.2 设置状态 (Settings State)

\`\`\`typescript { sendOnEnter: boolean, setSendOnEnter: (value) => void,
includeMetadataInExport: boolean, setIncludeMetadataInExport: (value) => void }
\`\`\`

#### 3.3 选择状态 (Selection State)

\`\`\`typescript { selectedFiles: Set<string>, toggleFileSelection: (fileId) =>
void, clearFileSelection: () => void, selectAllFiles: (fileIds) => void,
invertFileSelection: (fileIds) => void,

selectedMessages: Set<string>, toggleMessageSelection: (messageId) => void,
clearMessageSelection: () => void, selectAllMessages: (messageIds) => void,
invertMessageSelection: (messageIds) => void } \`\`\`

#### 3.4 房间状态 (Room State)

\`\`\`typescript { currentRoomId: string, setCurrentRoomId: (roomId) => void,
leftSidebarCollapsed: boolean, toggleLeftSidebar: () => void } \`\`\`

### 4. 组件通信架构

\`\`\` 全局状态 (Zustand Store) ↓ ┌───────────────────────────────────────────┐
│ 任何组件都可以订阅和更新全局状态 │
└───────────────────────────────────────────┘ ↓ 组件 A 更新状态 →
所有订阅该状态的组件自动更新 \`\`\`

**优点**：

- 无需 props drilling
- 状态更新自动触发重渲染
- 支持状态持久化（localStorage）
- 类型安全（TypeScript）

---

## 模块划分

### 1. 目录结构

\`\`\` elizabeth-frontend/ ├── app/ # Next.js App Router │ ├── layout.tsx #
根布局 │ ├── page.tsx # 主页面 │ └── globals.css # 全局样式 │ ├── components/ #
组件目录 │ ├── layout/ # 布局组件 │ │ ├── top-bar.tsx # 顶部导航栏 │ │ ├──
left-sidebar.tsx # 左侧边栏 │ │ ├── middle-column.tsx # 中间消息区域 │ │ └──
right-sidebar.tsx # 右侧文件区域 │ │ │ ├── room/ # 房间相关组件 │ │ ├──
room-settings-form.tsx # 房间设置表单 │ │ ├── room-permissions.tsx # 权限显示 │
│ ├── room-capacity.tsx # 容量显示 │ │ └── room-sharing.tsx # 分享功能 │ │ │ ├──
chat/ # 聊天相关组件 │ │ ├── message-list.tsx # 消息列表 │ │ ├──
message-bubble.tsx # 消息气泡 │ │ ├── message-input.tsx # 消息输入 │ │ ├──
markdown-toolbar.tsx # Markdown 工具栏 │ │ ├── markdown-renderer.tsx # Markdown
渲染器 │ │ ├── code-block.tsx # 代码块组件 │ │ └── custom-markdown-editor.tsx #
自定义编辑器 │ │ │ ├── files/ # 文件相关组件 │ │ ├── file-list-view.tsx #
文件列表视图 │ │ ├── file-card.tsx # 文件卡片 │ │ ├── file-upload-zone.tsx #
文件上传区域 │ │ └── file-preview-modal.tsx # 文件预览对话框 │ │ │ ├── ui/ # UI
基础组件 (shadcn/ui) │ │ ├── button.tsx │ │ ├── card.tsx │ │ ├── dialog.tsx │ │
├── input.tsx │ │ ├── select.tsx │ │ ├── checkbox.tsx │ │ ├── badge.tsx │ │ ├──
progress.tsx │ │ ├── scroll-area.tsx │ │ ├── toast.tsx │ │ ├── toaster.tsx │ │
└── ... │ │ │ ├── theme-switcher.tsx # 主题切换器 │ ├── settings-dialog.tsx #
设置对话框 │ └── providers.tsx # 全局 Provider │ ├── lib/ # 工具库 │ ├──
types.ts # 类型定义 │ ├── store.ts # Zustand 状态管理 │ ├── utils.ts # 工具函数
│ ├── utils/ │ │ └── format.ts # 格式化工具 │ └── hooks/ │ ├── use-theme.ts #
主题 Hook │ └── use-mobile.ts # 移动端检测 Hook │ ├── api/ # API 服务层（Mock）
│ ├── roomService.ts # 房间 API │ ├── fileService.ts # 文件 API │ └──
shareService.ts # 分享 API │ ├── docs/ # 文档目录 │ └──
FRONTEND_DOCUMENTATION.md # 本文档 │ ├── public/ # 静态资源 │ └── ... │ ├──
package.json # 依赖配置 ├── tsconfig.json # TypeScript 配置 ├── next.config.mjs

# Next.js 配置 └── README.md # 项目说明 \`\`\`

### 2. 模块职责

#### 2.1 布局模块 (Layout Module)

**位置**：`components/layout/`

**职责**：

- 定义应用的整体布局结构
- 管理各个区域的显示和隐藏
- 处理响应式布局

**组件**：

- `TopBar`：顶部导航栏，包含 Logo、操作按钮、主题切换器
- `LeftSidebar`：左侧边栏，包含房间控制功能
- `MiddleColumn`：中间消息区域，包含消息列表和编辑器
- `RightSidebar`：右侧文件区域，包含文件列表和上传功能

#### 2.2 房间模块 (Room Module)

**位置**：`components/room/`

**职责**：

- 管理房间设置
- 显示房间信息
- 处理房间分享

**组件**：

- `RoomSettingsForm`：房间设置表单
- `RoomPermissions`：权限显示组件
- `RoomCapacity`：容量显示组件
- `RoomSharing`：分享功能组件

#### 2.3 聊天模块 (Chat Module)

**位置**：`components/chat/`

**职责**：

- 显示和管理消息
- 提供 Markdown 编辑功能
- 处理消息的增删改查

**组件**：

- `MessageList`：消息列表容器
- `MessageBubble`：单条消息显示
- `MessageInput`：消息输入组件
- `MarkdownToolbar`：Markdown 工具栏
- `MarkdownRenderer`：Markdown 渲染器
- `CodeBlock`：代码块显示组件
- `CustomMarkdownEditor`：自定义 Markdown 编辑器

#### 2.4 文件模块 (File Module)

**位置**：`components/files/`

**职责**：

- 显示和管理文件
- 处理文件上传
- 提供文件预览

**组件**：

- `FileListView`：文件列表容器
- `FileCard`：单个文件卡片
- `FileUploadZone`：文件上传区域
- `FilePreviewModal`：文件预览对话框

#### 2.5 UI 基础模块 (UI Module)

**位置**：`components/ui/`

**职责**：

- 提供可复用的 UI 组件
- 统一视觉风格
- 基于 shadcn/ui 构建

**组件**：

- 按钮、输入框、对话框、卡片等基础组件
- 所有组件支持主题切换
- 所有组件支持无障碍访问

#### 2.6 状态管理模块 (State Module)

**位置**：`lib/store.ts`

**职责**：

- 管理全局状态
- 提供状态更新方法
- 支持状态持久化

**状态**：

- 主题状态
- 设置状态
- 选择状态
- 房间状态

#### 2.7 API 服务模块 (API Module)

**位置**：`api/`

**职责**：

- 封装 API 调用
- 处理请求和响应
- 提供 Mock 数据（当前）

**服务**：

- `roomService`：房间相关 API
- `fileService`：文件相关 API
- `shareService`：分享相关 API

#### 2.8 工具模块 (Utils Module)

**位置**：`lib/`

**职责**：

- 提供工具函数
- 定义类型
- 提供自定义 Hooks

**文件**：

- `types.ts`：TypeScript 类型定义
- `utils.ts`：通用工具函数
- `utils/format.ts`：格式化工具
- `hooks/use-theme.ts`：主题 Hook
- `hooks/use-mobile.ts`：移动端检测 Hook

---

## API 接口设计

### 1. API 设计原则

- **RESTful 风格**：使用标准的 HTTP 方法（GET, POST, PUT, DELETE）
- **统一响应格式**：所有 API 返回统一的 JSON 格式
- **错误处理**：使用标准的 HTTP 状态码和错误信息
- **认证方式**：基于房间 ID 的无状态认证（可扩展为 JWT）

### 2. 响应格式

#### 成功响应

\`\`\`typescript { success: true, data: any, message?: string } \`\`\`

#### 错误响应

\`\`\`typescript { success: false, error: { code: string, message: string,
details?: any } } \`\`\`

### 3. 房间相关 API

#### 3.1 获取房间详情

\`\`\` GET /api/rooms/:roomId \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**响应**： \`\`\`typescript { success: true, data: { id: string, currentSize:
number, // 当前使用空间（MB）maxSize: number, // 最大空间（MB）settings: {
expiresAt: string, // ISO 8601 格式 passwordProtected: boolean, password?:
string, // 仅管理员可见 maxViews: number }, permissions: string[] // ["read",
"edit", "share", "delete"] } } \`\`\`

#### 3.2 更新房间设置

\`\`\` PUT /api/rooms/:roomId/settings \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**请求体**： \`\`\`typescript { expiresAt?: string, passwordProtected?: boolean,
password?: string, maxViews?: number } \`\`\`

**响应**： \`\`\`typescript { success: true, message: "房间设置已更新" } \`\`\`

#### 3.3 验证房间密码

\`\`\` POST /api/rooms/:roomId/verify \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**请求体**： \`\`\`typescript { password: string } \`\`\`

**响应**： \`\`\`typescript { success: true, data: { token: string, // 访问令牌
expiresIn: number } } \`\`\`

### 4. 消息相关 API

#### 4.1 获取消息列表

\`\`\` GET /api/rooms/:roomId/messages \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID
- `limit` (查询参数，可选)：返回数量，默认 50
- `offset` (查询参数，可选)：偏移量，默认 0

**响应**： \`\`\`typescript { success: true, data: { messages: [ { id: string,
content: string, timestamp: string, // ISO 8601 格式 user: string } ], total:
number, hasMore: boolean } } \`\`\`

#### 4.2 发送消息

\`\`\` POST /api/rooms/:roomId/messages \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**请求体**： \`\`\`typescript { content: string } \`\`\`

**响应**： \`\`\`typescript { success: true, data: { id: string, content:
string, timestamp: string, user: string } } \`\`\`

#### 4.3 更新消息

\`\`\` PUT /api/rooms/:roomId/messages/:messageId \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID
- `messageId` (路径参数)：消息 ID

**请求体**： \`\`\`typescript { content: string } \`\`\`

**响应**： \`\`\`typescript { success: true, data: { id: string, content:
string, timestamp: string, user: string } } \`\`\`

#### 4.4 删除消息

\`\`\` DELETE /api/rooms/:roomId/messages/:messageId \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID
- `messageId` (路径参数)：消息 ID

**响应**： \`\`\`typescript { success: true, message: "消息已删除" } \`\`\`

### 5. 文件相关 API

#### 5.1 获取文件列表

\`\`\` GET /api/rooms/:roomId/files \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**响应**： \`\`\`typescript { success: true, data: { files: [ { id: string,
name: string, thumbnailUrl: string | null, size: number, // 字节 type: "image" |
"video" | "pdf" | "link" | "document", url: string, uploadedAt: string // ISO
8601 格式 } ] } } \`\`\`

#### 5.2 上传文件

\`\`\` POST /api/rooms/:roomId/files \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**请求体**：

- `Content-Type: multipart/form-data`
- `file`: 文件数据

**响应**： \`\`\`typescript { success: true, data: { id: string, name: string,
thumbnailUrl: string | null, size: number, type: string, url: string,
uploadedAt: string } } \`\`\`

#### 5.3 删除文件

\`\`\` DELETE /api/rooms/:roomId/files/:fileId \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID
- `fileId` (路径参数)：文件 ID

**响应**： \`\`\`typescript { success: true, message: "文件已删除" } \`\`\`

#### 5.4 批量下载文件

\`\`\` POST /api/rooms/:roomId/files/download \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**请求体**： \`\`\`typescript { fileIds: string[] } \`\`\`

**响应**：

- `Content-Type: application/zip`
- 二进制 ZIP 文件数据

### 6. 分享相关 API

#### 6.1 获取分享链接

\`\`\` GET /api/rooms/:roomId/share/link \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**响应**： \`\`\`typescript { success: true, data: { link: string //
完整的分享链接 } } \`\`\`

#### 6.2 获取二维码

\`\`\` GET /api/rooms/:roomId/share/qrcode \`\`\`

**请求参数**：

- `roomId` (路径参数)：房间 ID

**响应**：

- `Content-Type: image/png`
- 二进制图片数据（PNG 格式的二维码）

### 7. 错误码定义

| 错误码              | HTTP 状态码 | 说明                 |
| ------------------- | ----------- | -------------------- |
| `ROOM_NOT_FOUND`    | 404         | 房间不存在           |
| `ROOM_EXPIRED`      | 410         | 房间已过期           |
| `ROOM_FULL`         | 403         | 房间访问次数已达上限 |
| `INVALID_PASSWORD`  | 401         | 密码错误             |
| `UNAUTHORIZED`      | 401         | 未授权访问           |
| `PERMISSION_DENIED` | 403         | 权限不足             |
| `FILE_TOO_LARGE`    | 413         | 文件过大             |
| `STORAGE_FULL`      | 507         | 存储空间不足         |
| `INVALID_REQUEST`   | 400         | 请求参数错误         |
| `SERVER_ERROR`      | 500         | 服务器内部错误       |

### 8. Mock API 实现

当前前端项目中，所有 API 调用都使用 Mock 实现，位于 `api/` 目录下：

- **roomService.ts**：模拟房间相关 API
- **fileService.ts**：模拟文件相关 API
- **shareService.ts**：模拟分享相关 API

**Mock 特点**：

- 模拟网络延迟（300-1000ms）
- 返回符合接口规范的数据
- 在控制台输出调用日志
- 易于替换为真实 API

**替换为真实 API 的步骤**：

1. 保持接口签名不变
2. 将 Mock 实现替换为真实的 HTTP 请求
3. 使用 `fetch` 或 `axios` 发送请求
4. 处理错误和异常情况
5. 更新环境变量配置（API 基础 URL）

---

## 技术栈

### 1. 核心框架

- **React 19.2**：UI 框架
- **Next.js 16**：React 全栈框架
- **TypeScript 5.x**：类型安全

### 2. 状态管理

- **Zustand 5.x**：轻量级状态管理
- **zustand/middleware**：状态持久化

### 3. UI 组件库

- **shadcn/ui**：基于 Radix UI 的组件库
- **Radix UI**：无样式的可访问组件
- **Tailwind CSS v4**：原子化 CSS 框架

### 4. Markdown 相关

- **react-markdown**：Markdown 渲染
- **remark-gfm**：GitHub Flavored Markdown 支持
- **rehype-raw**：支持 HTML 标签

### 5. 文件处理

- **react-dropzone**：拖放上传

### 6. 图标

- **lucide-react**：图标库

### 7. 工具库

- **clsx**：条件类名合并
- **tailwind-merge**：Tailwind 类名合并
- **date-fns**：日期格式化

### 8. 开发工具

- **ESLint**：代码检查
- **Prettier**：代码格式化
- **TypeScript**：类型检查

---

## 总结

Elizabeth
前端应用是一个功能完整、设计精美的文件分享与协作平台。它采用现代化的技术栈，提供了直观的用户界面和流畅的操作体验。

### 主要特点

1. **清晰的三栏布局**：左侧房间控制、中间消息协作、右侧文件管理
2. **完整的 Markdown 支持**：富文本编辑、实时预览、代码高亮
3. **灵活的文件管理**：拖放上传、批量操作、文件预览
4. **精细的权限控制**：房间设置、访问限制、权限管理
5. **优秀的用户体验**：主题切换、Toast 通知、响应式设计
6. **模块化架构**：清晰的模块划分、可维护的代码结构
7. **完善的 API 设计**：RESTful 风格、统一响应格式、易于对接

### 后续工作

1. **后端对接**：将 Mock API 替换为真实的后端 API
2. **实时通信**：集成 WebSocket 实现实时消息推送
3. **性能优化**：虚拟滚动、懒加载、代码分割
4. **功能扩展**：
   - 用户系统（可选）
   - 文件版本管理
   - 协作编辑
   - 消息搜索
   - 文件搜索
5. **测试**：单元测试、集成测试、E2E 测试
6. **部署**：配置生产环境、CDN 加速、监控告警

---

**文档版本**：v1.0 **最后更新**：2025-01-XX **维护者**：Elizabeth 开发团队
