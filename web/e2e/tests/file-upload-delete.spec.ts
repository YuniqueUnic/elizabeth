import { expect, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
const API_BASE = "http://localhost:4092/api/v1";
const TEST_ROOM = "file-auto-test";
const TEST_ROOM_URL = `${BASE_URL}/${TEST_ROOM}`;

test.describe.configure({ mode: "serial" });

test.describe("文件上传和删除 - 自动化测试", () => {
    let smallFile: string;
    let mediumFile: string;
    let largeFile: string;
    let tokenInfo: { token: string; refresh_token?: string; expires_at: string };

    const createTestFile = (name: string, sizeInKB: number): string => {
        const testDir = path.join(__dirname, "../../test-files");
        if (!fs.existsSync(testDir)) {
            fs.mkdirSync(testDir, { recursive: true });
        }
        const filePath = path.join(testDir, name);
        const buffer = Buffer.alloc(sizeInKB * 1024);
        fs.writeFileSync(filePath, buffer);
        return filePath;
    };

    const bootstrapRoomPage = async (page: any): Promise<RoomPage> => {
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
        return roomPage;
    };

    test.beforeAll(async ({ request }) => {
        smallFile = createTestFile("small-test.txt", 100); // 100KB
        mediumFile = createTestFile("medium-test.bin", 2 * 1024); // 2MB
        largeFile = createTestFile("large-test.bin", 4 * 1024); // 4MB

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

    test.afterAll(() => {
        [smallFile, mediumFile, largeFile].forEach((file) => {
            if (file && fs.existsSync(file)) {
                fs.unlinkSync(file);
            }
        });
    });

    test("初始化 - 访问房间并验证 UI", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        await roomPage.clearAllFiles();

        const mainVisible = await page.locator("main").isVisible();
        expect(mainVisible).toBe(true);

        const asideCount = await page.locator("aside").count();
        expect(asideCount).toBeGreaterThan(0);

        const uploadZone = page.locator(".p-4.pt-2 input[type='file']");
        await expect(uploadZone).toHaveCount(1);
    });

    test("小文件上传测试", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        await roomPage.uploadFile(smallFile);

        const files = await roomPage.getFileList();
        expect(files).toContain("small-test.txt");
    });

    test("中等文件上传测试", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        await roomPage.uploadFile(mediumFile);

        await expect.poll(async () => (await roomPage.getFileList()).length, {
            timeout: 20_000,
        }).toBeGreaterThanOrEqual(2);
    });

    test("大文件上传测试", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        await roomPage.uploadFile(largeFile);

        await expect.poll(async () => (await roomPage.getFileList()).length, {
            timeout: 60_000,
        }).toBeGreaterThanOrEqual(3);
    });

    test("文件选择和批量操作", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        let files = await roomPage.getFileList();
        if (files.length === 0) {
            await roomPage.uploadFile(smallFile);
            files = await roomPage.getFileList();
        }

        const fileItems = page.locator(
            "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
        );

        if ((await fileItems.count()) > 0) {
            const firstCheckbox = fileItems.first().locator("[role='checkbox']");
            const isChecked = await firstCheckbox.isChecked();
            if (!isChecked) {
                await firstCheckbox.click();
            }
            expect(await firstCheckbox.isChecked()).toBe(true);
        }
    });

    test("文件删除测试", async ({ page }) => {
        const roomPage = await bootstrapRoomPage(page);
        const filesBefore = await roomPage.getFileList();
        expect(filesBefore.length).toBeGreaterThan(0);

        await roomPage.clearAllFiles();
        const filesAfter = await roomPage.getFileList();
        expect(filesAfter.length).toBe(0);
    });
});
