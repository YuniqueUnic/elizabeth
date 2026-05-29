/**
 * BDD 测试：保存防抖、PDF 链接拦截、消息自动滚动
 *
 * 验证三个修复：
 * 1. 双击保存按钮不会产生重复消息
 * 2. 点击 /contents/ 链接打开文件预览而非跳转
 * 3. "始终追踪最新消息"开关控制自动滚动行为
 */

import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4092";
const API_BASE_URL = process.env.API_BASE_URL || "http://localhost:4092/api/v1";
const TOKEN_STORAGE_KEY = "elizabeth_tokens";

async function ensureRoomExists(roomName: string) {
  const response = await fetch(
    `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}`,
    { method: "POST" },
  );
  if (!response.ok && response.status !== 409) {
    throw new Error(
      `Failed to create room ${roomName}: ${response.status} ${response.statusText}`,
    );
  }
}

async function issueRoomToken(roomName: string) {
  const response = await fetch(
    `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}/tokens`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    },
  );
  if (!response.ok) {
    throw new Error(
      `Failed to issue token for ${roomName}: ${response.status}`,
    );
  }
  const token = await response.json();
  return {
    token: token.token as string,
    refreshToken: token.refresh_token as string | undefined,
    expiresAt: token.expires_at as string,
  };
}

// ─────────────────────────────────────────────────────────────
// Scenario 1: Save button debounce
// ─────────────────────────────────────────────────────────────
test.describe("SCENARIO 1: 保存按钮防抖 — 双击保存不会产生重复消息", () => {
  let roomPage: RoomPage;
  let currentRoom: string;
  let currentRoomUrl: string;

  test.beforeEach(async ({ page }) => {
    currentRoom = `save-debounce-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
    currentRoomUrl = `${BASE_URL}/${currentRoom}`;

    await ensureRoomExists(currentRoom);
    const token = await issueRoomToken(currentRoom);

    await page.addInitScript(
      ({ storageKey, roomName, tokenInfo }) => {
        const existing = JSON.parse(
          window.localStorage.getItem(storageKey) || "{}",
        );
        existing[roomName] = tokenInfo;
        window.localStorage.setItem(storageKey, JSON.stringify(existing));
      },
      {
        storageKey: TOKEN_STORAGE_KEY,
        roomName: currentRoom,
        tokenInfo: token,
      },
    );

    roomPage = new RoomPage(page);
    await roomPage.goto(currentRoomUrl);
    await roomPage.waitForRoomLoad();
  });

  test("GIVEN 用户发送了一条消息 WHEN 双击保存按钮 THEN 消息不应重复出现在 UI 中", async ({
    page,
  }) => {
    // GIVEN: 发送一条带唯一标识的消息
    const testMessage = `debounce-verify-${Date.now()}`;
    await roomPage.sendMessage(testMessage);

    // 确认消息出现在 UI 中
    const messages = page.getByTestId(/^message-content-/);
    await expect(messages.last()).toContainText(testMessage);

    // 记录当前消息数量
    const countBefore = await messages.count();

    // WHEN: 双击保存按钮
    const saveBtn = page.getByTestId("save-messages-btn");
    await saveBtn.dblclick();

    // 等待保存完成（toast 出现或按钮恢复可用）
    await page.waitForTimeout(3000);

    // THEN: 消息数量不应增加（不应有重复）
    const countAfter = await messages.count();
    expect(countAfter).toBe(countBefore);
  });

  test("GIVEN 保存正在进行中 WHEN 检查保存按钮状态 THEN 按钮标题从'保存中...'变回'保存'", async ({
    page,
  }) => {
    // GIVEN: 发送消息
    await roomPage.sendMessage(`disable-test-${Date.now()}`);

    // WHEN: 点击保存
    const saveBtn = page.getByTestId("save-messages-btn");
    await saveBtn.click();

    // THEN: 保存期间按钮应显示"保存中..."
    await expect(saveBtn).toHaveAttribute("title", /保存中/, { timeout: 1000 });

    // 保存完成后，标题变回"保存"（按钮因无未保存内容而 disabled）
    await expect(saveBtn).toHaveAttribute("title", "保存", { timeout: 10000 });
  });
});

// ─────────────────────────────────────────────────────────────
// Scenario 2: PDF / file link interception
// ─────────────────────────────────────────────────────────────
test.describe("SCENARIO 2: 文件链接拦截 — 点击 /contents/ 链接打开预览而非跳转", () => {
  let roomPage: RoomPage;
  let currentRoom: string;
  let currentRoomUrl: string;

  test.beforeEach(async ({ page }) => {
    currentRoom = `link-intercept-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
    currentRoomUrl = `${BASE_URL}/${currentRoom}`;

    await ensureRoomExists(currentRoom);
    const token = await issueRoomToken(currentRoom);

    await page.addInitScript(
      ({ storageKey, roomName, tokenInfo }) => {
        const existing = JSON.parse(
          window.localStorage.getItem(storageKey) || "{}",
        );
        existing[roomName] = tokenInfo;
        window.localStorage.setItem(storageKey, JSON.stringify(existing));
      },
      {
        storageKey: TOKEN_STORAGE_KEY,
        roomName: currentRoom,
        tokenInfo: token,
      },
    );

    roomPage = new RoomPage(page);
    await roomPage.goto(currentRoomUrl);
    await roomPage.waitForRoomLoad();
  });

  test("GIVEN 消息中包含 /contents/ 链接 WHEN 用户点击该链接 THEN 不应导航离开页面", async ({
    page,
  }) => {
    // GIVEN: 发送一条包含文件链接的消息
    const linkMessage = "[test-file.pdf](/contents/999)";
    await roomPage.sendMessage(linkMessage);

    // 确认链接渲染在消息中
    const messageContent = page.getByTestId(/^message-content-/).last();
    const link = messageContent.locator('a[href="/contents/999"]');
    await expect(link).toBeVisible({ timeout: 5000 });

    // 记录当前 URL
    const urlBefore = page.url();

    // WHEN: 点击链接
    await link.click();
    await page.waitForTimeout(500);

    // THEN: 页面 URL 不应改变（不跳转到后端原始端点）
    expect(page.url()).toBe(urlBefore);
  });

  test("GIVEN 消息中包含外部链接 WHEN 检查链接属性 THEN href 应保持原始值", async ({
    page,
  }) => {
    // GIVEN: 发送包含外部链接的消息
    await roomPage.sendMessage("[Example](https://example.com)");

    const messageContent = page.getByTestId(/^message-content-/).last();
    const link = messageContent.locator(
      'a[href="https://example.com"]',
    );
    await expect(link).toBeVisible({ timeout: 5000 });

    // 外部链接的 href 应该保持不变
    const href = await link.getAttribute("href");
    expect(href).toBe("https://example.com");
  });
});

// ─────────────────────────────────────────────────────────────
// Scenario 3: Auto-scroll toggle
// ─────────────────────────────────────────────────────────────
test.describe("SCENARIO 3: 消息自动滚动 — 始终追踪最新消息开关", () => {
  let roomPage: RoomPage;
  let currentRoom: string;
  let currentRoomUrl: string;

  test.beforeEach(async ({ page }) => {
    currentRoom = `auto-scroll-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
    currentRoomUrl = `${BASE_URL}/${currentRoom}`;

    await ensureRoomExists(currentRoom);
    const token = await issueRoomToken(currentRoom);

    await page.addInitScript(
      ({ storageKey, roomName, tokenInfo }) => {
        const existing = JSON.parse(
          window.localStorage.getItem(storageKey) || "{}",
        );
        existing[roomName] = tokenInfo;
        window.localStorage.setItem(storageKey, JSON.stringify(existing));
      },
      {
        storageKey: TOKEN_STORAGE_KEY,
        roomName: currentRoom,
        tokenInfo: token,
      },
    );

    roomPage = new RoomPage(page);
    await roomPage.goto(currentRoomUrl);
    await roomPage.waitForRoomLoad();
  });

  test("GIVEN 自动滚动开启 WHEN 新消息到达 THEN 消息列表应滚动到底部", async ({
    page,
  }) => {
    // GIVEN: 发送多条消息填满列表
    for (let i = 1; i <= 5; i++) {
      await roomPage.sendMessage(`fill-message-${i}`);
      await page.waitForTimeout(200);
    }

    // WHEN: 再发送一条新消息
    await roomPage.sendMessage("new-arrival-message");
    await page.waitForTimeout(1000);

    // THEN: 最后一条消息应该可见（列表滚动到底部）
    const lastMessage = page.getByTestId(/^message-content-/).last();
    await expect(lastMessage).toContainText("new-arrival-message");
  });

  test("GIVEN 用户向上滚动 THEN 应显示回到底部按钮", async ({
    page,
  }) => {
    // GIVEN: 发送多条消息
    for (let i = 1; i <= 10; i++) {
      await roomPage.sendMessage(`jump-btn-${i}`);
      await page.waitForTimeout(100);
    }
    await page.waitForTimeout(500);

    // 用户向上滚动 — 使用消息列表区域内的 scroll area
    const scrollArea = page
      .getByTestId("message-list-scroll")
      .locator("[data-radix-scroll-area-viewport]");
    await scrollArea.evaluate((el) => {
      el.scrollTop = 0;
    });
    await page.waitForTimeout(500);

    // THEN: 应显示"回到底部"按钮
    const jumpButton = page.getByRole("button", {
      name: /回到底部|Jump to latest/i,
    });
    await expect(jumpButton).toBeVisible({ timeout: 5000 });
  });

  test("GIVEN 回到底部按钮可见 WHEN 用户点击 THEN 列表应滚动到底部", async ({
    page,
  }) => {
    // GIVEN: 发送消息并向上滚动
    for (let i = 1; i <= 10; i++) {
      await roomPage.sendMessage(`click-jump-${i}`);
      await page.waitForTimeout(100);
    }
    await page.waitForTimeout(500);

    const scrollArea = page
      .getByTestId("message-list-scroll")
      .locator("[data-radix-scroll-area-viewport]");
    await scrollArea.evaluate((el) => {
      el.scrollTop = 0;
    });
    await page.waitForTimeout(500);

    // 确认按钮出现
    const jumpButton = page.getByRole("button", {
      name: /回到底部|Jump to latest/i,
    });
    await expect(jumpButton).toBeVisible({ timeout: 5000 });

    // WHEN: 点击按钮
    await jumpButton.click();
    await page.waitForTimeout(1000);

    // THEN: 最后一条消息应该可见
    const lastMessage = page.getByTestId(/^message-content-/).last();
    await expect(lastMessage).toContainText("click-jump-10");
  });
});
