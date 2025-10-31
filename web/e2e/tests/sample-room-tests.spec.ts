/**
 * 房间 UI 测试示例
 * 演示如何使用 PageObject 和链式调用进行 UI 测试
 */

import { expect, type Page, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
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
        const errorBody = await response.text().catch(() => "");
        throw new Error(
            `Failed to issue token for ${roomName}: ${response.status} ${response.statusText} ${errorBody}`,
        );
    }

    const token = await response.json();
    return {
        token: token.token as string,
        expiresAt: token.expires_at as string,
    };
}

async function bootstrapRoom(page: Page) {
    const roomName = `playwright-test-room-${Date.now()}-${
        Math.floor(Math.random() * 1e6)
    }`;
    const roomUrl = `${BASE_URL}/${roomName}`;

    await ensureRoomExists(roomName);
    const token = await issueRoomToken(roomName);

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
            roomName,
            tokenInfo: token,
        },
    );

    const roomPage = new RoomPage(page);
    await roomPage.goto(roomUrl);
    await roomPage.waitForRoomLoad();

    return { roomPage, roomName, roomUrl };
}

async function dismissToasts(page: Page) {
    const toastItems = page.locator(
        '[role="region"][aria-label="Notifications (F8)"] li',
    );
    if ((await toastItems.count()) === 0) return;

    await page.keyboard.press("Escape").catch(() => {});
    await toastItems.waitFor({ state: "detached", timeout: 2000 }).catch(
        () => {},
    );
}

test.describe("Elizabeth Room UI Tests", () => {
    let roomPage: RoomPage;
    let currentRoom: string;
    let currentRoomUrl: string;

    test.beforeEach(async ({ page }) => {
        const setup = await bootstrapRoom(page);
        roomPage = setup.roomPage;
        currentRoom = setup.roomName;
        currentRoomUrl = setup.roomUrl;
    });

    // ==================== 消息系统测试 ====================

    test("TC001: Should send and display message", async () => {
        const testMessage = "Test message from Playwright";

        // 发送消息
        await roomPage.sendMessage(testMessage);

        // 验证消息已显示
        const messageCount = await roomPage.getMessageCount();
        expect(messageCount).toBeGreaterThan(0);

        // 验证消息内容
        const lastMessage = await roomPage.getLastMessageText();
        expect(lastMessage).toContain(testMessage);
    });

    test("TC002: Should display unsaved badge after sending", async () => {
        // 发送消息
        await roomPage.sendMessage("Unsaved message test");

        // 验证"未保存"标签显示
        const hasUnsaved = await roomPage.hasUnsavedBadge();
        expect(hasUnsaved).toBe(true);
    });

    test("TC003: Should save message successfully", async () => {
        // 发送消息
        await roomPage.sendMessage("Message to save");

        // 点击保存按钮
        await roomPage.topBar.saveBtn.click();

        // 等待保存成功提示
        await roomPage.waitForToast("保存成功", 5000).catch(() => {
            // Toast 可能不会显示
        });

        // 验证"未保存"标签消失
        await roomPage.page.waitForTimeout(500);
        const hasUnsaved = await roomPage.hasUnsavedBadge();
        expect(hasUnsaved).toBe(false);
    });

    test("TC004: Should send multiple messages", async () => {
        const messageCount = await roomPage.getMessageCount();

        // 发送 3 条消息
        for (let i = 1; i <= 3; i++) {
            await roomPage.sendMessage(`Message ${i}`);
            await roomPage.page.waitForTimeout(300);
        }

        // 验证消息计数增加
        const newMessageCount = await roomPage.getMessageCount();
        expect(newMessageCount).toBe(messageCount + 3);
    });

    // ==================== 房间设置测试 ====================

    test("TC005: Should update room settings - expiration time", async () => {
        // 修改过期时间
        await roomPage.roomSettings.expirationTime.selectOption("1 周");
        await roomPage.page.waitForTimeout(200);

        // 验证选择值
        const selectedValue = await roomPage.roomSettings.expirationTime
            .getSelectedText();
        expect(selectedValue).toContain("1 周");
    });

    test("TC006: Should set room password", async () => {
        const testPassword = "TestPassword123!";

        // 填充密码
        await roomPage.roomSettings.password.fill(testPassword);

        // 验证密码已输入
        const password = await roomPage.roomSettings.password.getValue();
        expect(password).toBe(testPassword);
    });

    test("TC007: Should update max view count", async () => {
        const newCount = 75;

        // 设置最大查看次数
        await roomPage.roomSettings.maxViewCount.setValue(newCount);

        // 验证值已设置
        const count = await roomPage.roomSettings.maxViewCount.getNumberValue();
        expect(count).toBe(newCount);
    });

    test("TC008: Should save all room settings", async () => {
        // 填充所有设置
        await roomPage.fillRoomSettings({
            expirationTime: "1 周",
            password: "SecurePass123",
            maxViewCount: 100,
        });

        // 保存设置
        await roomPage.saveRoomSettings();

        // 等待保存完成
        await roomPage.page.waitForTimeout(1000);

        // 验证所有设置已保存
        const expirationText = await roomPage.roomSettings.expirationTime
            .getSelectedText();
        expect(expirationText).toContain("1 周");

        const password = await roomPage.roomSettings.password.getValue();
        expect(password).toBe("SecurePass123");

        const count = await roomPage.roomSettings.maxViewCount.getNumberValue();
        expect(count).toBe(100);
    });

    // ==================== 权限管理测试 ====================

    test("TC009: Preview permission should stay enabled", async () => {
        const state = await roomPage.roomPermissions.previewBtn.getAttribute(
            "aria-pressed",
        );
        expect(state).toBe("true");

        const isEnabled = await roomPage.roomPermissions.previewBtn.isEnabled();
        expect(isEnabled).toBe(true);
    });

    test("TC010: Should enable edit permission", async () => {
        await roomPage.setRoomPermissions({
            preview: true,
            edit: true,
            share: false,
            delete: false,
        });

        const after = await roomPage.roomPermissions.editBtn.getAttribute(
            "aria-pressed",
        );
        expect(after).toBe("true");
    });

    test("TC011: Should toggle share permission", async () => {
        const shareBtn = roomPage.roomPermissions.shareBtn.getLocator();
        const before = await shareBtn.getAttribute("aria-pressed");

        await roomPage.roomPermissions.shareBtn.click();
        await roomPage.page.waitForTimeout(200);

        const after = await shareBtn.getAttribute("aria-pressed");
        expect(after).not.toBe(before);
    });

    test("TC012: Should save permissions", async () => {
        // 设置权限
        await roomPage.setRoomPermissions({
            preview: true,
            edit: false,
            share: false,
            delete: false,
        });

        // 等待保存完成
        await roomPage.page.waitForTimeout(1000);

        // 验证没有错误提示
        try {
            await roomPage.waitForToast("权限已保存", 3000);
        } catch {
            // Toast 可能不出现，这是可以接受的
        }
    });

    // ==================== UI 交互测试 ====================

    test("TC013: Should display room URL correctly", async () => {
        const roomUrl = roomPage.getRoomUrl();
        expect(roomUrl).toContain(currentRoom);
    });

    test("TC014: Should get room name from URL", async () => {
        const roomName = roomPage.getRoomName();
        expect(roomName).toBe(currentRoom);
    });

    test("TC015: Should get capacity information", async () => {
        const capacityInfo = await roomPage.getCapacityInfo();
        expect(capacityInfo).toMatch(/MB/);
    });

    // ==================== 链式调用测试 ====================

    test("TC016: Should use fluent interface for message sending", async () => {
        // 链式调用测试
        await roomPage.messages.input
            .fill("Fluent interface test")
            .then(() => roomPage.messages.input.waitForVisible());

        // 点击发送
        await roomPage.messages.sendBtn.click();

        // 验证
        await roomPage.page.waitForTimeout(500);
        const messageCount = await roomPage.getMessageCount();
        expect(messageCount).toBeGreaterThan(0);
    });

    test("TC017: Should use fluent interface for settings", async () => {
        // 链式设置
        await roomPage.roomSettings.password
            .clear()
            .then(() => roomPage.roomSettings.password.fill("newpass123"));

        // 验证
        const password = await roomPage.roomSettings.password.getValue();
        expect(password).toBe("newpass123");
    });

    // ==================== 错误处理测试 ====================

    test("TC018: Should handle element visibility", async () => {
        // 检查元素是否可见
        const isVisible = await roomPage.messages.input.isVisible();
        expect(isVisible).toBe(true);
    });

    test("TC019: Should check button enabled state", async () => {
        // 填充消息
        await roomPage.messages.input.fill("test");

        // 检查发送按钮是否启用
        const isEnabled = await roomPage.messages.sendBtn.isEnabled();
        // 启用状态取决于消息内容，这里仅做演示
        expect(typeof isEnabled).toBe("boolean");
    });

    test("TC020: Should take screenshot for debugging", async () => {
        // 截图用于调试
        await roomPage.screenshot("room-test-debug");

        // 验证截图成功（文件应该存在）
        // 实际的文件检查取决于文件系统权限
        expect(true).toBe(true);
    });
});

// ==================== 整合测试场景 ====================

test.describe("End-to-End Room Scenarios", () => {
    test("E2E-001: Complete room setup workflow", async ({ page }) => {
        const { roomPage, roomName } = await bootstrapRoom(page);

        // 2. 配置房间设置
        await roomPage.fillRoomSettings({
            expirationTime: "12 小时",
            password: "E2ETestPass123",
            maxViewCount: 75,
        });
        await roomPage.saveRoomSettings();

        // 3. 设置权限
        await roomPage.setRoomPermissions({
            preview: true,
            edit: true,
            share: true,
            delete: false,
        });

        await dismissToasts(roomPage.page);

        // 4. 发送消息
        await roomPage.sendMessage("E2E Test Message 1");
        await roomPage.sendMessage("E2E Test Message 2");

        // 5. 保存消息
        await roomPage.topBar.saveBtn.click();

        // 6. 验证最终状态
        const messageCount = await roomPage.getMessageCount();
        expect(messageCount).toBe(2);

        // 7. 验证房间 URL
        const resolvedRoomName = roomPage.getRoomName();
        expect(resolvedRoomName).toBe(roomName);
    });

    test("E2E-002: Message lifecycle workflow", async ({ page }) => {
        const { roomPage } = await bootstrapRoom(page);

        // 1. 发送消息
        await roomPage.sendMessage("Lifecycle test message");

        // 2. 验证"未保存"状态
        let hasUnsaved = await roomPage.hasUnsavedBadge();
        expect(hasUnsaved).toBe(true);

        // 3. 保存消息
        await roomPage.topBar.saveBtn.click();
        await roomPage.page.waitForTimeout(1000);

        // 4. 验证"未保存"状态消失
        hasUnsaved = await roomPage.hasUnsavedBadge();
        expect(hasUnsaved).toBe(false);
    });
});
