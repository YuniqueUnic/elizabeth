# Playwright UI 测试架构设计

## 📋 架构概览

```
web/e2e/
├── fixtures/
│   ├── base.fixture.ts           # 基础 fixture
│   └── app.fixture.ts            # 应用 fixture
├── page-objects/
│   ├── base.page.ts              # 基础 PageObject
│   ├── room-page.ts              # 房间页面 PageObject
│   └── index.ts                  # 导出
├── selectors/
│   ├── html-selectors.ts         # HTML 元素选择器映射（核心文件）
│   └── locators.ts               # Locator 工具函数
├── tests/
│   ├── room-creation.spec.ts     # 房间创建测试
│   ├── messaging.spec.ts         # 消息系统测试
│   ├── room-settings.spec.ts     # 房间设置测试
│   ├── permissions.spec.ts       # 权限管理测试
│   └── file-operations.spec.ts   # 文件操作测试
└── playwright.config.ts
```

## 🏗️ 核心设计模式

### 1. HTML 元素映射结构 (html-selectors.ts)

```typescript
// 树形结构，对应 UI 层级
const selectors = {
  topBar: {
    saveBtn: 'button[name="save"]',
    copyBtn: 'button[name="copy"]',
    downloadBtn: 'button[name="download"]',
    deleteBtn: 'button[name="delete"]',
  },
  leftSidebar: {
    roomSettings: {
      section: '[data-testid="room-settings"]',
      expirationTime: 'combobox[aria-label="过期时间"]',
      password: 'input[name="password"]',
      passwordToggle: 'button[aria-label="toggle-password"]',
      maxViewCount: 'input[name="max_times"]',
      saveBtn: 'button[name="save-settings"]',
    },
    roomPermissions: {
      section: '[data-testid="room-permissions"]',
      previewBtn: 'button[name="perm-preview"]',
      editBtn: 'button[name="perm-edit"]',
      shareBtn: 'button[name="perm-share"]',
      deleteBtn: 'button[name="perm-delete"]',
      saveBtn: 'button[name="save-permissions"]',
    },
    roomSharing: {
      section: '[data-testid="room-sharing"]',
      getLink: 'button[name="get-link"]',
      download: 'button[name="download-qr"]',
    },
  },
  middleColumn: {
    messageInput: 'textarea[placeholder*="输入消息"]',
    sendBtn: 'button[name="send-message"]',
    messageList: '[data-testid="message-list"]',
    messageItem: ".message-item",
    unsavedBadge: ".unsaved-badge",
  },
  rightSidebar: {
    fileUpload: 'button[name="upload-file"]',
    fileList: '[data-testid="file-list"]',
    fileItem: ".file-item",
  },
};
```

### 2. PageObject 设计

```typescript
// 链式调用支持
class RoomPage {
  readonly page: Page;
  readonly selectors: typeof selectors;

  constructor(page: Page) {
    this.page = page;
    this.selectors = selectors;
  }

  // 支持链式调用的属性
  get roomSettings() {
    return {
      expirationTime: new ComboboxElement(
        this.page,
        this.selectors.leftSidebar.roomSettings.expirationTime
      ),
      password: new InputElement(
        this.page,
        this.selectors.leftSidebar.roomSettings.password
      ),
      maxViewCount: new SpinbuttonElement(...),
      saveBtn: new ButtonElement(...),
    };
  }

  get roomPermissions() { ... }
  get middleColumn() { ... }
  get topBar() { ... }
}
```

### 3. 元素类（支持方法链）

```typescript
class BaseElement {
  constructor(protected page: Page, protected selector: string) {}

  async click() {
    await this.page.click(this.selector);
    return this;
  }

  async fill(text: string) {
    await this.page.fill(this.selector, text);
    return this;
  }
}

class InputElement extends BaseElement {
  async fill(text: string) {
    // 实现
    return this;
  }

  async clear() {
    await this.page.fill(this.selector, "");
    return this;
  }
}
```

## 📝 测试场景覆盖

### 1. 房间创建测试

- [ ] 创建公开房间
- [ ] 创建密码保护房间
- [ ] 验证房间 URL 生成

### 2. 消息系统测试

- [ ] 发送消息
- [ ] 消息显示未保存状态
- [ ] 保存消息
- [ ] 编辑消息
- [ ] 删除消息

### 3. 房间设置测试

- [ ] 修改过期时间
- [ ] 设置房间密码
- [ ] 修改最大查看次数
- [ ] 保存设置

### 4. 权限管理测试

- [ ] 切换权限开关
- [ ] 验证权限依赖关系
- [ ] 保存权限

### 5. 文件操作测试

- [ ] 上传文件
- [ ] 删除文件
- [ ] 批量下载文件

## 🎯 使用示例

```typescript
// 测试文件中的使用
test("should save room settings", async ({ page }) => {
  const roomPage = new RoomPage(page);

  // 链式调用示例
  await roomPage.roomSettings.expirationTime
    .click()
    .then(() => roomPage.roomSettings.expirationTime.selectOption("1 周"));

  await roomPage.roomSettings.password
    .fill("test123")
    .then(() => roomPage.roomSettings.maxViewCount.fill("50"));

  await roomPage.roomSettings.saveBtn.click();

  // 验证成功
  await expect(page.locator("text=保存成功")).toBeVisible();
});
```
