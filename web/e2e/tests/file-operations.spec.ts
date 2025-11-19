/**
 * 文件操作 UI 测试
 * 包括文件上传、显示、删除等功能
 */

import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";
import htmlSelectors from "../selectors/html-selectors";
import * as fs from "fs";
import * as path from "path";

const BASE_URL = "http://localhost:4001";
const TEST_ROOM = "file-test-room";
const TEST_ROOM_URL = `${BASE_URL}/${TEST_ROOM}`;
const API_BASE = "http://localhost:4092/api/v1";

// 创建测试文件
function createTestFile(name: string, content: string): string {
    const testDir = path.join(__dirname, "../../test-files");
    if (!fs.existsSync(testDir)) {
        fs.mkdirSync(testDir, { recursive: true });
    }
    const filePath = path.join(testDir, name);
    fs.writeFileSync(filePath, content);
    return filePath;
}

test.describe("文件操作测试", () => {
    let roomPage: RoomPage;
    const testFiles: string[] = [];
    let tokenInfo: { token: string; refresh_token?: string; expires_at: string };

    test.beforeAll(async ({ request }) => {
        // 创建测试文件
        testFiles.push(createTestFile("test1.txt", "Test file 1 content"));
        testFiles.push(
            createTestFile("test2.md", "# Test File 2\n\nMarkdown content"),
        );
        testFiles.push(
            createTestFile(
                "test3.json",
                JSON.stringify({ test: "data" }, null, 2),
            ),
        );

        // 确保房间存在（已存在则忽略）
        const createResp = await request.post(
            `${API_BASE}/rooms/${TEST_ROOM}?password=`,
            { data: {}, timeout: 15_000 },
        );
        if (!createResp.ok() && createResp.status() !== 409) {
            throw new Error(
                `创建文件房间失败，status=${createResp.status()}`,
            );
        }

        // 预获取 token 用于后续复用，避免频繁鉴权/限流
        const tokenResp = await request.post(
            `${API_BASE}/rooms/${TEST_ROOM}/tokens`,
            {
                data: { password: "", with_refresh_token: true },
                timeout: 15_000,
            },
        );
        if (!tokenResp.ok()) {
            throw new Error(
                `无法获取文件房间访问令牌，status=${tokenResp.status()}`,
            );
        }
        tokenInfo = await tokenResp.json();
    });

    test.beforeEach(async ({ page }) => {
        roomPage = new RoomPage(page);

        // 预注入 token，避免首次加载 401/429
        await page.addInitScript(
            ({ roomName, token, refreshToken, expiresAt }) => {
                const storageKey = "elizabeth_tokens";
                const existing =
                    JSON.parse(window.localStorage.getItem(storageKey) || "{}") ||
                    {};
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

        await roomPage.goto(TEST_ROOM_URL);
        await roomPage.waitForRoomLoad();
        await roomPage.clearAllFiles();
    });

    test.afterAll(() => {
        // 清理测试文件
        testFiles.forEach((filePath) => {
            if (fs.existsSync(filePath)) {
                fs.unlinkSync(filePath);
            }
        });
    });

    // ==================== 文件上传测试 ====================

    test("FILE-001: 应该成功上传单个文件", async () => {
        // 上传文件
        await roomPage.uploadFile(testFiles[0]);

        // 验证文件列表中出现该文件
        const files = await roomPage.getFileList();
        expect(files).toContain("test1.txt");
    });

    test("FILE-002: 应该成功上传多个文件", async () => {
        // 上传多个文件
        await roomPage.uploadMultipleFiles([testFiles[0], testFiles[1]]);

        // 验证所有文件都出现在列表中
        const files = await roomPage.getFileList();
        expect(files).toContain("test1.txt");
        expect(files).toContain("test2.md");
    });

    test("FILE-003: 应该显示正确的文件数量", async () => {
        // 上传 3 个文件
        await roomPage.uploadMultipleFiles(testFiles);

        // 获取文件列表
        const files = await roomPage.getFileList();
        expect(files.length).toBe(3);
    });

    test("FILE-004: 应该显示文件大小", async ({ page }) => {
        // 上传文件
        await roomPage.uploadFile(testFiles[0]);

        // 查找文件大小信息
        const sizeInfo = page.locator("text=/B|KB|MB/").first();
        const isVisible = await sizeInfo.isVisible().catch(() => false);
        expect(isVisible).toBe(true);
    });

    // ==================== 文件删除测试 ====================

    test("FILE-005: 应该成功删除文件", async () => {
        // 上传文件
        await roomPage.uploadFile(testFiles[0]);

        // 验证文件存在
        let files = await roomPage.getFileList();
        expect(files).toContain("test1.txt");

        // 删除文件
        await roomPage.deleteFile("test1.txt");

        // 验证文件已被删除
        files = await roomPage.getFileList();
        expect(files).not.toContain("test1.txt");
    });

    test("FILE-006: 应该删除多个文件后文件列表为空", async () => {
        // 上传一个文件
        await roomPage.uploadFile(testFiles[0]);

        // 验证文件存在
        let files = await roomPage.getFileList();
        expect(files.length).toBeGreaterThan(0);

        // 删除所有文件
        for (const file of files) {
            await roomPage.deleteFile(file);
        }

        // 验证文件列表为空
        files = await roomPage.getFileList();
        expect(files.length).toBe(0);
    });

    // ==================== 文件交互测试 ====================

    test("FILE-007: 应该选择文件", async ({ page }) => {
        // 上传文件
        await roomPage.uploadFile(testFiles[0]);

        // 查找文件复选框并点击
        const checkboxes = page.locator(
            `${htmlSelectors.rightSidebar.fileManager.fileList.fileItem.container} ${htmlSelectors.rightSidebar.fileManager.fileList.fileItem.checkbox}`,
        );
        const count = await checkboxes.count();
        if (count > 0) {
            await checkboxes.first().click();

            // 验证复选框被选中
            const isChecked = await checkboxes.first().isChecked();
            expect(isChecked).toBe(true);
        }
    });

    test("FILE-008: 应该全选文件", async () => {
        // 上传多个文件
        await roomPage.uploadMultipleFiles([testFiles[0], testFiles[1]]);

        // 点击全选按钮
        await roomPage.files.selectAllBtn.click();
        await roomPage.page.waitForTimeout(300);

        // 验证所有复选框都被选中
        const checkboxes = roomPage.page.locator(
            `${htmlSelectors.rightSidebar.fileManager.fileList.fileItem.container} ${htmlSelectors.rightSidebar.fileManager.fileList.fileItem.checkbox}`,
        );
        let allChecked = true;
        const count = await checkboxes.count();
        for (let i = 0; i < count; i++) {
            const isChecked = await checkboxes.nth(i).isChecked();
            if (!isChecked) {
                allChecked = false;
                break;
            }
        }
        expect(allChecked).toBe(true);
    });

    // ==================== 文件显示测试 ====================

    test("FILE-009: 空文件列表应该显示提示消息", async ({ page }) => {
        // 不上传任何文件，检查是否显示"暂无文件"
        const emptyStateText = page.locator('text="暂无文件"');
        const isVisible = await emptyStateText.isVisible().catch(() => false);
        expect(isVisible).toBe(true);
    });

    test("FILE-010: 文件上传后应该隐藏空状态提示", async ({ page }) => {
        // 上传文件
        await roomPage.uploadFile(testFiles[0]);

        // 检查"暂无文件"是否隐藏
        const emptyStateText = page.locator('text="暂无文件"');
        const isVisible = await emptyStateText.isVisible().catch(() => false);
        expect(isVisible).toBe(false);

        // 检查文件列表是否可见
        const fileItems = page.locator(
            htmlSelectors.rightSidebar.fileManager.fileList.fileItem.container,
        );
        expect(await fileItems.count()).toBeGreaterThan(0);
    });

    // ==================== 错误处理测试 ====================

    test("FILE-011: 删除不存在的文件应该抛出错误", async () => {
        // 尝试删除不存在的文件
        let errorThrown = false;
        try {
            await roomPage.deleteFile("nonexistent-file.txt");
        } catch (error) {
            errorThrown = true;
        }
        expect(errorThrown).toBe(true);
    });
});
