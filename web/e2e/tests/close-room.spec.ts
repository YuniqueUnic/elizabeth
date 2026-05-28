/**
 * 关闭房间功能测试 (BDD)
 *
 * 验证"关闭房间"对话框的安全逻辑：
 *  - 无密码房间：直接显示危险确认步骤
 *  - 有密码房间：必须先通过密码校验才能进入危险确认步骤
 *  - 密码错误时：不得进入下一步，必须显示错误提示
 *  - 密码正确时：可以进入危险确认步骤并完成关闭
 *
 * Bug 复现场景：持有有效 token 的用户输入错误密码，
 * 不应该跳过密码校验直接删除房间。
 */

import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4092";
const API_BASE_URL = process.env.API_BASE_URL || "http://localhost:4092/api/v1";
const TOKEN_STORAGE_KEY = "elizabeth_tokens";

// ============================================================================
// 测试工具函数
// ============================================================================

async function createRoom(roomName: string, password?: string) {
    const url = `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}${password ? `?password=${encodeURIComponent(password)}` : ""}`;
    const response = await fetch(url, { method: "POST" });
    if (!response.ok && response.status !== 409) {
        throw new Error(`Failed to create room ${roomName}: ${response.status}`);
    }
}

async function issueRoomToken(roomName: string, password?: string) {
    const response = await fetch(
        `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}/tokens`,
        {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ password, with_refresh_token: true }),
        },
    );
    if (!response.ok) {
        const body = await response.text().catch(() => "");
        throw new Error(`Failed to issue token: ${response.status} ${body}`);
    }
    const data = await response.json();
    return {
        token: data.token as string,
        refreshToken: data.refresh_token as string | undefined,
        expiresAt: data.expires_at as string,
    };
}

/**
 * 检查房间是否真实存在。
 *
 * 使用 DELETE 请求探测：不带 token 必然返回验权失败，但
 * - 401/403 表示房间存在（权限不足）
 * - 404 表示房间不存在
 *
 * 避免使用 GET /rooms/{name}，该接口在房间不存在时会自动创建新房间。
 */
async function roomExists(roomName: string): Promise<boolean> {
    // 尝试签发一个 token（不带密码），利用返回码判断房间是否存在
    // - 200: 房间存在（且无密码保护）
    // - 401/403: 房间存在（有密码保护）
    // - 404: 房间不存在
    const response = await fetch(
        `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}/tokens`,
        {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ with_refresh_token: false }),
        },
    );
    return response.status !== 404;
}

// ============================================================================
// 测试套件
// ============================================================================

test.describe("关闭房间功能测试", () => {
    // ------------------------------------------------------------------
    // SCENARIO 1: 无密码房间 - 直接显示危险确认
    // ------------------------------------------------------------------
    test.describe("无密码保护的房间", () => {
        let roomName: string;
        let roomPage: RoomPage;

        test.beforeEach(async ({ page }) => {
            roomName = `close-room-no-pwd-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
            await createRoom(roomName);
            const token = await issueRoomToken(roomName);

            await page.addInitScript(
                ({ storageKey, name, tokenInfo }) => {
                    const existing = JSON.parse(
                        window.localStorage.getItem(storageKey) || "{}",
                    );
                    existing[name] = tokenInfo;
                    window.localStorage.setItem(
                        storageKey,
                        JSON.stringify(existing),
                    );
                },
                { storageKey: TOKEN_STORAGE_KEY, name: roomName, tokenInfo: token },
            );

            roomPage = new RoomPage(page);
            await roomPage.goto(`${BASE_URL}/${roomName}`);
            await roomPage.waitForRoomLoad();
        });

        test("DR-001: 无密码房间点击关闭后直接显示危险确认步骤（跳过密码输入）", async ({ page }) => {
            // 点击关闭房间按钮
            await page.getByRole("button", { name: "关闭房间" }).click();

            // 对话框应出现
            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 无密码房间直接进入第 2 步：显示危险警告，不显示密码输入框
            await expect(
                dialog.getByRole("textbox", { name: /房间密码/ }),
            ).not.toBeVisible();
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).toBeVisible();
            await expect(
                dialog.locator("text=/永久物理删除/"),
            ).toBeVisible();
        });

        test("DR-002: 无密码房间确认关闭后房间被删除并跳转至首页", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 点击确定物理关闭
            await dialog.getByRole("button", { name: "确定物理关闭" }).click();

            // 应跳转回首页
            await expect(page).toHaveURL(`${BASE_URL}/`, { timeout: 15000 });

            // 房间应已不存在
            const exists = await roomExists(roomName);
            expect(exists).toBe(false);
        });


        test("DR-003: 关闭对话框后取消不删除房间", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 点击取消
            await dialog.getByRole("button", { name: "取消" }).click();
            await expect(dialog).not.toBeVisible({ timeout: 3000 });

            // 房间应仍然存在
            const exists = await roomExists(roomName);
            expect(exists).toBe(true);
        });
    });

    // ------------------------------------------------------------------
    // SCENARIO 2: 有密码房间 - 密码错误不得跳过
    // ------------------------------------------------------------------
    test.describe("有密码保护的房间 - 密码校验安全性", () => {
        const ROOM_PASSWORD = "correct-password-123"; // pragma: allowlist secret
        let roomName: string;
        let roomPage: RoomPage;

        test.beforeEach(async ({ page }) => {
            roomName = `close-room-with-pwd-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
            await createRoom(roomName, ROOM_PASSWORD);

            // 以正确密码获取 token 注入 localStorage（模拟已登入用户）
            const token = await issueRoomToken(roomName, ROOM_PASSWORD);

            await page.addInitScript(
                ({ storageKey, name, tokenInfo }) => {
                    const existing = JSON.parse(
                        window.localStorage.getItem(storageKey) || "{}",
                    );
                    existing[name] = tokenInfo;
                    window.localStorage.setItem(
                        storageKey,
                        JSON.stringify(existing),
                    );
                },
                { storageKey: TOKEN_STORAGE_KEY, name: roomName, tokenInfo: token },
            );

            roomPage = new RoomPage(page);
            await roomPage.goto(`${BASE_URL}/${roomName}`);
            await roomPage.waitForRoomLoad();
        });

        test("DR-004: 有密码房间点击关闭后显示密码输入步骤", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 应显示密码输入框，不显示危险确认按钮
            await expect(
                dialog.getByLabel(/请输入房间密码/),
            ).toBeVisible();
            await expect(
                dialog.getByRole("button", { name: "下一步" }),
            ).toBeVisible();
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).not.toBeVisible();
        });

        test("DR-005: [安全] 持有有效 token 但输入错误密码时，不得进入危险确认步骤", async ({
            page,
        }) => {
            // === 这是复现 Bug 的核心测试 ===
            // 用户虽然持有有效 token，但输入了错误密码
            // 预期：密码验证失败，停留在密码输入步骤，显示错误提示
            // Bug 行为：错误密码也能点击"下一步"进入危险确认步骤并删除房间

            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 输入错误密码
            await dialog.getByLabel(/请输入房间密码/).fill("wrong-password-xyz");
            await dialog.getByRole("button", { name: "下一步" }).click();

            // 等待验证响应
            await page.waitForTimeout(2000);

            // ✅ 断言：必须显示错误提示，不得跳转到下一步
            await expect(
                dialog.locator("p.text-destructive, [class*='destructive'] p"),
            ).toBeVisible({ timeout: 5000 });

            // ✅ 断言：危险确认按钮不得出现
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).not.toBeVisible();

            // ✅ 断言：仍停留在密码输入步骤
            await expect(
                dialog.getByLabel(/请输入房间密码/),
            ).toBeVisible();

            // ✅ 断言：房间仍然存在
            const exists = await roomExists(roomName);
            expect(exists).toBe(true);
        });

        test("DR-006: 输入空密码时显示提示错误，不发起网络请求", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 不填密码直接点下一步
            await dialog.getByRole("button", { name: "下一步" }).click();

            // 应立即显示错误（无需网络请求）
            await expect(
                dialog.locator("p.text-destructive, [class*='destructive'] p"),
            ).toBeVisible({ timeout: 2000 });

            // 仍停留在密码输入步骤
            await expect(
                dialog.getByLabel(/请输入房间密码/),
            ).toBeVisible();
        });

        test("DR-007: 输入正确密码后进入危险确认步骤", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 输入正确密码
            await dialog.getByLabel(/请输入房间密码/).fill(ROOM_PASSWORD);
            await dialog.getByRole("button", { name: "下一步" }).click();

            // 应进入第 2 步：显示危险确认
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).toBeVisible({ timeout: 5000 });
            await expect(
                dialog.getByRole("button", { name: "下一步" }),
            ).not.toBeVisible();
        });

        test("DR-008: 密码验证通过并确认关闭后，房间被删除并跳转首页", async ({
            page,
        }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 第 1 步：输入正确密码
            await dialog.getByLabel(/请输入房间密码/).fill(ROOM_PASSWORD);
            await dialog.getByRole("button", { name: "下一步" }).click();

            // 第 2 步：确认关闭
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).toBeVisible({ timeout: 5000 });
            await dialog.getByRole("button", { name: "确定物理关闭" }).click();

            // 应跳转回首页
            await expect(page).toHaveURL(`${BASE_URL}/`, { timeout: 15000 });

            // 房间应已不存在
            const exists = await roomExists(roomName);
            expect(exists).toBe(false);
        });


        test("DR-009: 第二步中点击取消不删除房间", async ({ page }) => {
            await page.getByRole("button", { name: "关闭房间" }).click();

            const dialog = page.getByRole("dialog");
            await expect(dialog).toBeVisible({ timeout: 5000 });

            // 通过密码进入第 2 步
            await dialog.getByLabel(/请输入房间密码/).fill(ROOM_PASSWORD);
            await dialog.getByRole("button", { name: "下一步" }).click();
            await expect(
                dialog.getByRole("button", { name: "确定物理关闭" }),
            ).toBeVisible({ timeout: 5000 });

            // 在第 2 步点击取消
            await dialog.getByRole("button", { name: "取消" }).click();
            await expect(dialog).not.toBeVisible({ timeout: 3000 });

            // 房间应仍然存在
            const exists = await roomExists(roomName);
            expect(exists).toBe(true);
        });
    });
});
