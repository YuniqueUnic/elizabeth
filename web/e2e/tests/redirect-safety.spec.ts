
import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";
import * as fs from "fs";
import * as path from "path";

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
        throw new Error(
            `Failed to issue token for ${roomName}: ${response.status} ${response.statusText}`,
        );
    }
    const token = await response.json();
    return {
        token: token.token as string,
        expiresAt: token.expires_at as string,
    };
}

test.describe("房间重定向安全检查", () => {
    let roomPage: RoomPage;
    let currentRoom: string;
    let currentRoomUrl: string;

    test.beforeEach(async ({ page }) => {
        currentRoom = `redirect-test-${Date.now()}`;
        currentRoomUrl = `${BASE_URL}/${currentRoom}`;

        await ensureRoomExists(currentRoom);
        const token = await issueRoomToken(currentRoom);

        // Inject token
        await page.addInitScript(
            ({ storageKey, roomName, tokenInfo }) => {
                const existing = JSON.parse(
                    window.localStorage.getItem(storageKey) || "{}",
                );
                existing[roomName] = tokenInfo;
                window.localStorage.setItem(
                    storageKey,
                    JSON.stringify(existing),
                );
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

    test("当房间地址变更且有未保存内容时，应显示警告", async ({ page }) => {
        // 1. 发送一条消息（产生未保存状态）
        // 这里的 sendMessage 是 UI 操作，会在界面上显示 "未保存"
        await roomPage.sendMessage("This is an unsaved message");

        // 验证界面上确实有 "未保存" 标记 (依赖页面具体实现，查找包含 "未保存" 的元素)
        await expect(page.locator("text=未保存").first()).toBeVisible();

        // 2. 拦截 Settings 更新请求，伪造 Slug 变更
        // 监听 PUT /api/v1/rooms/*/settings
        const newSlug = `${currentRoom}-renamed`;
        console.log(`Expecting route interception for room: ${currentRoom}`);

        // Debug all requests
        page.on('request', request => console.log('>>', request.method(), request.url()));

        // Use regex for more robust matching
        await page.route(/\/settings/, async (route) => {
            const url = route.request().url();
            console.log(`Route intercepted! URL: ${url}`);

            if (route.request().method() !== "PUT") {
                return route.continue();
            }
            const response = await route.fetch();
            const json = await response.json();

            // 伪造返回数据中的 slug
            const modifiedJson = {
                ...json,
                slug: newSlug,
                name: newSlug,
            };

            await route.fulfill({
                response,
                json: modifiedJson,
            });
        });

        // 3. 修改房间设置并保存
        await roomPage.roomSettings.maxViewCount.setValue(999);
        await roomPage.roomSettings.saveBtn.click();

        // 等待保存完成 (Toast 出现)
        await expect(page.locator("text=配置已保存").first()).toBeVisible();

        // 4. 验证 Alert 出现
        // 注意：Alert 可能被模态框遮挡，但 DOM 中应存在且可见 (Playwright visible definition)
        // 尝试关闭模态框以确保可见 (点击遮罩层或按 ESC)
        await page.keyboard.press("Escape");

        const alert = page.locator("div[role='alert']").filter({ hasText: "房间地址已变更" });
        await expect(alert).toBeVisible({ timeout: 5000 });

        // 验证警告文本
        // "检测到你有未保存的消息更改或正在进行的上传"
        await expect(alert).toContainText("检测到你有未保存的消息更改或正在进行的上传");

        // 验证新地址显示
        await expect(alert).toContainText(`/${newSlug}`);
    });

    test("当修改权限导致房间地址变更时，应显示警告", async ({ page }) => {
        // 1. 发送未保存消息
        await roomPage.sendMessage("Message before permission change");

        // 2. 拦截 Permissions 更新请求，伪造 Slug 变更
        // Permissions update uses POST /rooms/:name/permissions
        const newSlug = `${currentRoom}-perm-renamed`;
        await page.route(/\/permissions/, async (route) => {
            if (route.request().method() !== "POST") return route.continue();

            const response = await route.fetch();
            const json = await response.json();

            await route.fulfill({
                response,
                json: { ...json, slug: newSlug, name: newSlug }
            });
        });

        // 3. 修改权限并保存
        // 切换一个权限，例如关闭分享
        await roomPage.roomPermissions.shareBtn.click();
        await roomPage.roomPermissions.saveBtn.click();

        // 4. 验证 Alert 出现
        const alert = page.locator("div[role='alert']").filter({ hasText: "房间地址已变更" });
        await expect(alert).toBeVisible({ timeout: 5000 });

        // 验证包含未保存内容的警告
        await expect(alert).toContainText("检测到你有未保存的消息更改");
        await expect(alert).toContainText(`/${newSlug}`);
    });
});
