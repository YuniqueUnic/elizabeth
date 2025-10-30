# Playwright UI 测试框架指南

## 📋 架构概览

```
web/e2e/
├── selectors/
│   └── html-selectors.ts          ← 核心：HTML 元素映射（树形结构）
├── page-objects/
│   ├── base-element.ts             ← 元素基类（支持链式调用）
│   ├── base-page.ts                ← PageObject 基类
│   └── room-page.ts                ← 房间页面 PageObject
├── fixtures/
│   ├── base.fixture.ts
│   └── app.fixture.ts
├── tests/
│   ├── room-creation.spec.ts
│   ├── messaging.spec.ts
│   ├── room-settings.spec.ts
│   ├── permissions.spec.ts
│   └── file-operations.spec.ts
└── playwright.config.ts
```

## 🏗️ 核心设计

### 1. HTML 选择器映射 (html-selectors.ts)

**特点**:

- 树形结构对应 UI 层级
- 集中维护所有选择器
- 易于更新和维护

```typescript
// 使用示例
const selectors = htmlSelectors.leftSidebar.roomSettings.password.input;
// 输出：'input[placeholder="设置房间密码"]'
```

### 2. 元素包装类 (base-element.ts)

**特点**:

- 支持链式调用（Fluent Interface）
- 提供通用的操作方法
- 返回 `this` 支持链式调用

```typescript
// 链式调用示例
await element
  .fill("text")
  .click()
  .waitForVisible()
  .screenshot();
```

### 3. PageObject 模式 (room-page.ts)

**特点**:

- 封装页面特定的操作
- 提供高级操作方法
- 支持链式导航

```typescript
// 使用示例
await roomPage.roomSettings.password
  .fill("test123")
  .then(() => roomPage.roomSettings.saveBtn.click());
```

## 💡 使用方式

### 基础操作

```typescript
import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

test("should send message", async ({ page }) => {
  const roomPage = new RoomPage(page);

  // 导航到房间
  await page.goto("http://localhost:4001/refactor-test");
  await roomPage.waitForRoomLoad();

  // 发送消息
  await roomPage.messages.input.fill("Hello World");
  await roomPage.messages.sendBtn.click();

  // 验证
  const lastMessage = await roomPage.getLastMessageText();
  expect(lastMessage).toBe("Hello World");
});
```

### 链式调用

```typescript
// 直接链式调用
await roomPage.messages.input
  .fill("test message")
  .then((el) => el.getLocator().press("Enter"));

// 或使用 async/await
await roomPage.roomSettings.password.fill("test123");
await roomPage.roomSettings.maxViewCount.setValue(50);
await roomPage.roomSettings.saveBtn.click();
```

### 高级操作

```typescript
// 房间设置
await roomPage.fillRoomSettings({
  expirationTime: "1 周",
  password: "secure123", // pragma: allowlist secret
  maxViewCount: 100,
});
await roomPage.saveRoomSettings();

// 权限设置
await roomPage.setRoomPermissions({
  preview: true,
  edit: true,
  share: false,
  delete: false,
});
```

## 📊 测试场景示例

### 场景 1: 房间创建和消息发送

```typescript
test("room creation and messaging", async ({ page }) => {
  const roomPage = new RoomPage(page);

  // 导航到房间
  await roomPage.goto("http://localhost:4001/test-room");
  await roomPage.waitForRoomLoad();

  // 发送消息
  await roomPage.sendMessage("Test message 1");
  expect(await roomPage.getMessageCount()).toBe(1);

  await roomPage.sendMessage("Test message 2");
  expect(await roomPage.getMessageCount()).toBe(2);

  // 验证最后一条消息
  const lastMsg = await roomPage.getLastMessageText();
  expect(lastMsg).toContain("Test message 2");
});
```

### 场景 2: 房间设置

```typescript
test("room settings configuration", async ({ page }) => {
  const roomPage = new RoomPage(page);

  await roomPage.waitForRoomLoad();

  // 修改设置
  await roomPage.fillRoomSettings({
    expirationTime: "1 周",
    password: "test123", // pragma: allowlist secret
    maxViewCount: 50,
  });

  // 保存
  await roomPage.saveRoomSettings();

  // 验证 Toast
  await roomPage.waitForToast("保存成功");
});
```

### 场景 3: 权限管理

```typescript
test("room permissions management", async ({ page }) => {
  const roomPage = new RoomPage(page);

  await roomPage.waitForRoomLoad();

  // 设置权限
  await roomPage.setRoomPermissions({
    preview: true,
    edit: false,
    share: false,
    delete: false,
  });

  // 验证保存成功
  await roomPage.waitForToast("权限已保存");
});
```

## 🔧 添加新的选择器

当 UI 发生变化时，更新 `html-selectors.ts`:

```typescript
// html-selectors.ts
export const htmlSelectors = {
  // ... 现有代码 ...

  // 添加新的部分
  newSection: {
    element1: 'button:has-text("新按钮")',
    element2: 'input[name="new-input"]',
  },
};
```

然后在 PageObject 中使用：

```typescript
// room-page.ts
get newSection() {
  return {
    element1: new ButtonElement(this.page, htmlSelectors.newSection.element1),
    element2: new InputElement(this.page, htmlSelectors.newSection.element2),
  };
}
```

## 🎯 最佳实践

### 1. 使用有意义的选择器

```typescript
// 好
button:has-text("保存设置")
input[placeholder="设置房间密码"]
text=/共.*条消息/

// 不好
button >> nth=5
div >> nth=1
span >> nth=0
```

### 2. 等待元素就绪

```typescript
// 使用 waitForVisible 而不是 sleep
await element.waitForVisible();

// 不推荐
await page.waitForTimeout(1000);
```

### 3. 错误处理

```typescript
// 好
try {
  await element.waitForVisible(2000);
} catch {
  // 元素不可见
}

// 不好
await element.click(); // 可能抛出错误
```

### 4. 链式调用

```typescript
// 好：链式调用简洁清晰
await roomPage.messages.input.fill("text");
await roomPage.messages.sendBtn.click();

// 也可以
const input = roomPage.messages.input;
await input.fill("text");
await input.press("Enter");
```

## 📝 编写测试的步骤

1. **导入必要的模块**
   ```typescript
   import { expect, test } from "@playwright/test";
   import { RoomPage } from "../page-objects/room-page";
   ```

2. **创建 PageObject 实例**
   ```typescript
   const roomPage = new RoomPage(page);
   ```

3. **执行操作**
   ```typescript
   await roomPage.messages.input.fill("message");
   ```

4. **验证结果**
   ```typescript
   expect(await roomPage.getMessageCount()).toBe(1);
   ```

## 🐛 调试技巧

### 打印 HTML 快照

```typescript
await roomPage.printSnapshot();
```

### 截图

```typescript
await roomPage.screenshot("test-name");
```

### 获取元素文本

```typescript
const text = await roomPage.messages.input.getText();
console.log(text);
```

### 使用 Playwright Inspector

```bash
npx playwright test --debug
```

## 🚀 运行测试

```bash
# 运行所有测试
npx playwright test

# 运行特定文件
npx playwright test tests/room-settings.spec.ts

# 运行特定测试
npx playwright test -g "should send message"

# 以 UI 模式运行
npx playwright test --ui

# 以调试模式运行
npx playwright test --debug
```

## ✅ 测试检查清单

- [ ] 选择器已添加到 `html-selectors.ts`
- [ ] PageObject 中已添加对应的 getter
- [ ] 高级操作方法已完成
- [ ] 测试脚本已编写
- [ ] 测试已运行并通过
- [ ] 截图已添加
- [ ] 文档已更新

---

**最后更新**: 2025-10-30 **作者**: QA Automation Team
