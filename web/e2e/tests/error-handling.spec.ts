
import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
const API_BASE_URL = process.env.API_BASE_URL || "http://localhost:4092/api/v1";

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
    const token = await response.json();
    return {
        token: token.token as string,
        expiresAt: token.expires_at as string,
    };
}

test.describe("异常处理与边界条件测试", () => {
    test("访问非法房间名称应显示错误提示", async ({ page }) => {
        // 非法名称：以连字符开头，或者包含非法字符
        const invalidRoomName = "-invalid-name";
        await page.goto(`${BASE_URL}/${invalidRoomName}`);

        // Use a more specific locator to avoid matching Next.js route announcer
        const alert = page.locator("div[role='alert'][data-slot='alert']");
        await expect(alert).toBeVisible();
        await expect(alert).toContainText("房间名称不合法");
    });

    test.skip("发送消息网络失败应显示错误提示", async ({ page }) => {
        // Listen to console logs
        page.on("console", msg => console.log(`[Browser Console] ${msg.text()}`));

        const roomName = `error-test-${Date.now()}`;
        await ensureRoomExists(roomName);
        const token = await issueRoomToken(roomName);

        // Inject token
        await page.addInitScript(
            ({ roomName, tokenInfo }) => {
                const existing = JSON.parse(
                    window.localStorage.getItem("elizabeth_tokens") || "{}",
                );
                existing[roomName] = tokenInfo;
                window.localStorage.setItem("elizabeth_tokens", JSON.stringify(existing));
            },
            { roomName, tokenInfo: token },
        );

        const roomPage = new RoomPage(page);
        await roomPage.goto(`${BASE_URL}/${roomName}`);
        await roomPage.waitForRoomLoad();

        // 拦截消息发送请求并强制失败
        await page.route(`**/api/v1/rooms/${roomName}/messages`, (route) => {
            console.log("Intercepted message request:", route.request().url());
            if (route.request().method() === "POST") {
                console.log("Aborting POST request");
                route.abort("failed");
            } else {
                route.continue();
            }
        });

        // 尝试发送消息
        await roomPage.messages.input.fill("This message will fail");
        await roomPage.messages.sendBtn.click();

        // 触发保存操作以发起网络请求
        const saveBtn = page.getByTestId("save-messages-btn");
        await expect(saveBtn).toBeEnabled();

        // Use evaluate to force click in JS
        await saveBtn.evaluate((btn) => (btn as HTMLElement).click());

        // 验证错误 Toast
        // Toast 通常包含 "失败" 或 "Error"
        // 使用更宽泛的匹配，因为具体文案可能不同
        // Search for text anywhere first to debug
        // Increase timeout to account for retries (default 3 retries with delay)
        // Also check for any toast that is open
        const toast = page.locator("[data-state='open']").filter({ hasText: /失败|错误|Error|Fail/ });

        // Debug: if not visible, print all open toasts
        if (!await toast.isVisible({ timeout: 5000 })) {
             const allToasts = page.locator("[data-state='open']");
             const count = await allToasts.count();
             console.log(`Found ${count} open toasts.`);
             for(let i=0; i<count; i++) {
                 console.log(`Toast ${i}:`, await allToasts.nth(i).innerText());
             }
        }

        await expect(toast).toBeVisible({ timeout: 15000 });
    });
});
