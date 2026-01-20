
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

test.describe("房间生命周期与限制测试", () => {
    let currentRoom: string;
    let currentRoomUrl: string;

    test.beforeEach(async () => {
        currentRoom = `lifecycle-test-${Date.now()}`;
        currentRoomUrl = `${BASE_URL}/${currentRoom}`;
        await ensureRoomExists(currentRoom);
    });

    test("当房间达到最大查看次数时，后续访问应被拒绝", async ({ browser }) => {
        // 1. 管理员上下文：设置最大查看次数
        const adminContext = await browser.newContext();
        const adminPage = await adminContext.newPage();
        const roomPage = new RoomPage(adminPage);

        // 注入 Token (模拟管理员/创建者)
        const token = await issueRoomToken(currentRoom);
        await adminPage.addInitScript(
            ({ roomName, tokenInfo }) => {
                const existing = JSON.parse(
                    window.localStorage.getItem("elizabeth_tokens") || "{}",
                );
                existing[roomName] = tokenInfo;
                window.localStorage.setItem("elizabeth_tokens", JSON.stringify(existing));
            },
            { roomName: currentRoom, tokenInfo: token },
        );

        await roomPage.goto(currentRoomUrl);
        await roomPage.waitForRoomLoad();

        // 设置最大查看次数为 2 (管理员本次访问算 1 次？)
        // 注意：后端逻辑通常是 "check limit -> enter -> increment".
        // 如果当前已经进入，修改 limit 为当前次数，下一次应该失败。
        // 或者修改 limit 为 2，再访问 1 次。

        // 我们设置 Max Views = 2
        await roomPage.fillRoomSettings({ maxViewCount: 2 });
        await roomPage.saveRoomSettings();

        await adminPage.close();

        // 2. 访客 A：消耗第 2 次访问机会
        const visitorAContext = await browser.newContext();
        const visitorAPage = await visitorAContext.newPage();
        const visitorARoomPage = new RoomPage(visitorAPage);

        // 访客直接访问（无 Token，会自动申请 Token 并消耗次数）
        await visitorARoomPage.goto(currentRoomUrl);
        await visitorARoomPage.waitForRoomLoad();

        // 验证访问成功
        await expect(visitorARoomPage.messages.input.getLocator()).toBeVisible();
        await visitorAPage.close();

        // 3. 访客 B：尝试第 3 次访问，应失败
        const visitorBContext = await browser.newContext();
        const visitorBPage = await visitorBContext.newPage();

        await visitorBPage.goto(currentRoomUrl);

        // 验证显示错误信息
        // 错误信息在 Alert 中："房间无法通过该链接进入：可能已过期、达到最大进入次数..."
        // Use data-slot="alert" to avoid matching Next.js route announcer
        const errorAlert = visitorBPage.locator("div[role='alert'][data-slot='alert']");
        await expect(errorAlert).toBeVisible({ timeout: 10000 });
        await expect(errorAlert).toContainText("达到最大进入次数");

        await visitorBPage.close();
    });

    test.skip("房间过期后应无法访问", async ({ browser }) => {
        // 使用 API 快速设置过期时间为 1 秒后
        // 因为 UI 最小选项是 1 分钟，测试等待太久。
        // 我们模拟 API 请求来修改设置。

        const adminContext = await browser.newContext();
        const adminPage = await adminContext.newPage();
        const roomPage = new RoomPage(adminPage);

        // 注入 Token
        const token = await issueRoomToken(currentRoom);
        await adminPage.addInitScript(
            ({ roomName, tokenInfo }) => {
                const existing = JSON.parse(
                    window.localStorage.getItem("elizabeth_tokens") || "{}",
                );
                existing[roomName] = tokenInfo;
                window.localStorage.setItem("elizabeth_tokens", JSON.stringify(existing));
            },
            { roomName: currentRoom, tokenInfo: token },
        );

        await roomPage.goto(currentRoomUrl);
        await roomPage.waitForRoomLoad();

        // 拦截并修改设置请求，或者直接调用 API
        // 这里我们直接调用 API 修改过期时间
        // 设置为过去的时间，确保立即过期
        // Note: Backend might return 500 "expired unexpectedly" but the room should be expired.
        const past = new Date(Date.now() - 10000); // 10 seconds ago
        const expireTime = past.toISOString().split(".")[0]; // YYYY-MM-DDTHH:mm:ss

        console.log(`Attempting to set expiry to: ${expireTime}`);

        // 使用 page.request 上下文发送 API 请求
        const apiResponse = await adminPage.request.put(`${API_BASE_URL}/rooms/${currentRoom}/settings?token=${token.token}`, {
            headers: {
                "Content-Type": "application/json"
            },
            data: {
                expire_at: expireTime
            }
        });

        // Expect 500 or 200 depending on backend behavior, but main goal is expiration
        if (!apiResponse.ok()) {
             console.log(`Backend returned ${apiResponse.status()} (expected for past expiry): ${await apiResponse.text()}`);
        } else {
             console.log("Backend accepted past expiry with 200 OK");
        }

        // No wait needed
        // await adminPage.waitForTimeout(5000);

        // 刷新页面或新开页面验证
        await adminPage.reload();

        // Debug: check current URL
        console.log("Current URL after reload:", adminPage.url());

        // Check if we are redirected to home
        if (adminPage.url() === `${BASE_URL}/`) {
            console.log("Redirected to home, likely room expired and not accessible");
            // If redirected to home, we might see an error toast or just be on home page
            // Adjust expectation accordingly.
        }

        // 验证显示过期错误
        // Use a new visitor context to verify expiration, as the admin token might still be valid or allow access
        const visitorContext = await browser.newContext();
        const visitorPage = await visitorContext.newPage();
        await visitorPage.goto(currentRoomUrl);

        // Debug: check room details directly
        const checkResponse = await visitorPage.request.get(`${API_BASE_URL}/rooms/${currentRoom}`);
        const roomDetails = await checkResponse.json();
        console.log("Room details from API:", JSON.stringify(roomDetails, null, 2));

        const errorAlert = visitorPage.locator("div[role='alert'][data-slot='alert']");
        await expect(errorAlert).toBeVisible({ timeout: 10000 });
        await expect(errorAlert).toContainText("可能已过期");

        await visitorPage.close();
        await adminPage.close();
    });
});
