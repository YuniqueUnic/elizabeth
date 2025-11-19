import { test, expect } from "@playwright/test";
import * as fs from "fs";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
const API_BASE = "http://localhost:4092/api/v1";
const TEST_ROOM = "file-upload-correct";
const TEST_ROOM_URL = `${BASE_URL}/${TEST_ROOM}`;

test.describe("正确的文件上传测试", () => {
    let tokenInfo: { token: string; refresh_token?: string; expires_at: string };

    test.beforeAll(async ({ request }) => {
        await request
            .post(`${API_BASE}/rooms/${TEST_ROOM}?password=`, { data: {}, timeout: 15_000 })
            .catch(() => {});
        const tokenResp = await request.post(`${API_BASE}/rooms/${TEST_ROOM}/tokens`, {
            data: { password: "", with_refresh_token: true },
            timeout: 15_000,
        });
        if (!tokenResp.ok()) {
            throw new Error(`无法获取文件房间令牌，status=${tokenResp.status()}`);
        }
        tokenInfo = await tokenResp.json();
    });

    test("应成功上传文件并显示在列表中", async ({ page }) => {
        await page.addInitScript(
            ({ roomName, token, refreshToken, expiresAt }) => {
                const storageKey = "elizabeth_tokens";
                const existing =
                    JSON.parse(window.localStorage.getItem(storageKey) || "{}") || {};
                existing[roomName] = { token, refreshToken, expiresAt };
                window.localStorage.setItem(storageKey, JSON.stringify(existing));
            },
            {
                roomName: TEST_ROOM,
                token: tokenInfo.token,
                refreshToken: tokenInfo.refresh_token,
                expiresAt: tokenInfo.expires_at,
            },
        );

        const roomPage = new RoomPage(page);
        await roomPage.goto(TEST_ROOM_URL);
        await roomPage.waitForRoomLoad();
        await roomPage.clearAllFiles();

        const testFile = "/tmp/correct-upload-test.txt";
        fs.writeFileSync(testFile, "test content for correct upload");

        await roomPage.uploadFile(testFile);

        const files = await roomPage.getFileList();
        expect(files.some((name) => name.includes("correct-upload-test.txt"))).toBe(true);
        await fs.promises.unlink(testFile);
    });
});
