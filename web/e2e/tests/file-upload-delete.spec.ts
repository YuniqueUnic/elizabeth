/**
 * 文件上传和删除功能自动化测试
 *
 * 测试场景：
 * 1. 小文件上传 (<1MB)
 * 2. 中等文件上传 (5MB)
 * 3. 大文件上传 (15MB)
 * 4. 文件删除
 * 5. 批量操作
 */

import { expect, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

const BASE_URL = "http://localhost:4001";
const TEST_ROOM = "file-auto-test-" + Date.now();
const TEST_ROOM_URL = `${BASE_URL}/${TEST_ROOM}`;

// 创建测试文件
function createTestFile(name: string, sizeInKB: number): string {
    const testDir = path.join(__dirname, "../../test-files");
    if (!fs.existsSync(testDir)) {
        fs.mkdirSync(testDir, { recursive: true });
    }
    const filePath = path.join(testDir, name);
    const buffer = Buffer.alloc(sizeInKB * 1024);
    fs.writeFileSync(filePath, buffer);
    return filePath;
}

test.describe("文件上传和删除 - 自动化测试", () => {
    let smallFile: string;
    let mediumFile: string;
    let largeFile: string;

    test.beforeAll(() => {
        // 创建测试文件
        smallFile = createTestFile("small-test.txt", 100); // 100KB
        mediumFile = createTestFile("medium-test.bin", 5 * 1024); // 5MB
        largeFile = createTestFile("large-test.bin", 15 * 1024); // 15MB

        console.log("✓ 测试文件已创建");
        console.log(`  - 小文件：${smallFile} (100KB)`);
        console.log(`  - 中等文件：${mediumFile} (5MB)`);
        console.log(`  - 大文件：${largeFile} (15MB)`);
    });

    test.afterAll(() => {
        // 清理测试文件
        [smallFile, mediumFile, largeFile].forEach((file) => {
            if (fs.existsSync(file)) {
                fs.unlinkSync(file);
            }
        });
    });

    test("初始化 - 访问房间并验证 UI", async ({ page }) => {
        console.log("\n========== 初始化测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 验证页面加载
        const main = page.locator("main");
        await expect(main).toBeVisible();

        // 验证右侧栏
        const aside = page.locator("aside");
        const asideCount = await aside.count();
        console.log(`✓ 找到 ${asideCount} 个 aside 容器`);
        expect(asideCount).toBeGreaterThan(0);

        // 验证文件输入存在
        const fileInput = page.locator("input[type='file']");
        await expect(fileInput).toHaveCount(1);
        console.log("✓ 文件输入元素已找到");
    });

    test("小文件上传测试", async ({ page }) => {
        console.log("\n========== 小文件上传测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 等待页面完全加载
        await page.waitForTimeout(1000);

        // 上传文件
        console.log(`上传：small-test.txt (100KB)`);
        const fileInput = page.locator("input[type='file']");
        await fileInput.setInputFiles(smallFile);

        // 等待上传完成 (小文件应该很快)
        await page.waitForTimeout(2000);

        // 验证文件出现在列表中
        const fileItems = page.locator(
            "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
        );
        let itemCount = await fileItems.count();
        console.log(`✓ 文件列表中有 ${itemCount} 项`);
        expect(itemCount).toBeGreaterThan(0);

        // 获取文件名
        const firstItem = fileItems.first();
        const fileName = await firstItem.locator(".file-name").textContent();
        console.log(`✓ 上传的文件：${fileName}`);
        expect(fileName).toContain("small-test.txt");
    });

    test("中等文件上传测试", async ({ page }) => {
        console.log("\n========== 中等文件上传测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 等待页面完全加载
        await page.waitForTimeout(1000);

        // 上传文件
        console.log(`上传：medium-test.bin (5MB)`);
        const fileInput = page.locator("input[type='file']");
        const startTime = Date.now();

        await fileInput.setInputFiles(mediumFile);

        // 等待上传完成 (中等文件可能需要几秒)
        const maxWait = 15000;
        let uploaded = false;

        for (let i = 0; i < maxWait / 500; i++) {
            const fileItems = page.locator(
                "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
            );
            const itemCount = await fileItems.count();

            if (itemCount >= 2) { // 应该有小文件 + 中等文件
                uploaded = true;
                const elapsed = Math.round((Date.now() - startTime) / 1000);
                console.log(`✓ 中等文件上传完成 (耗时：${elapsed}s)`);
                break;
            }

            if (i % 4 === 0) {
                console.log(`  等待中... (${i / 2}s)`);
            }
            await page.waitForTimeout(500);
        }

        expect(uploaded).toBe(true);
    });

    test("大文件上传测试", async ({ page }) => {
        console.log("\n========== 大文件上传测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 等待页面完全加载
        await page.waitForTimeout(1000);

        // 上传文件
        console.log(`上传：large-test.bin (15MB)`);
        const fileInput = page.locator("input[type='file']");
        const startTime = Date.now();

        await fileInput.setInputFiles(largeFile);

        // 等待上传完成 (大文件可能需要 30 秒左右)
        const maxWait = 60000;
        let uploaded = false;

        for (let i = 0; i < maxWait / 1000; i++) {
            const fileItems = page.locator(
                "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
            );
            const itemCount = await fileItems.count();

            if (itemCount >= 3) { // 应该有小文件 + 中等文件 + 大文件
                uploaded = true;
                const elapsed = Math.round((Date.now() - startTime) / 1000);
                console.log(`✓ 大文件上传完成 (耗时：${elapsed}s)`);
                break;
            }

            if (i % 5 === 0) {
                console.log(`  等待中... (${i}s)`);
            }
            await page.waitForTimeout(1000);
        }

        expect(uploaded).toBe(true);
    });

    test("文件删除测试", async ({ page }) => {
        console.log("\n========== 文件删除测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 等待页面完全加载
        await page.waitForTimeout(1000);

        // 获取文件列表
        const fileItems = page.locator(
            "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
        );
        let initialCount = await fileItems.count();
        console.log(`✓ 初始文件数：${initialCount}`);
        expect(initialCount).toBeGreaterThan(0);

        // 删除第一个文件
        const firstItem = fileItems.first();
        const fileName = await firstItem.locator(".file-name").textContent();
        console.log(`准备删除：${fileName}`);

        // 鼠标悬停显示删除按钮
        await firstItem.hover();
        await page.waitForTimeout(300);

        // 点击删除按钮
        const deleteBtn = firstItem.locator("button[title='删除文件']");
        await deleteBtn.click();
        console.log("✓ 删除按钮已点击");

        // 等待确认对话框或直接执行删除
        await page.waitForTimeout(500);

        // 检查是否有确认对话框
        const dialog = page.locator("[role='dialog']");
        const hasDialog = await dialog.isVisible().catch(() => false);

        if (hasDialog) {
            console.log("✓ 确认对话框出现");
            // 点击确认按钮
            const confirmBtn = dialog.locator("button").filter({
                hasText: /确定|删除|确认/,
            }).first();
            await confirmBtn.click();
            console.log("✓ 确认删除");
        }

        // 等待删除完成
        await page.waitForTimeout(1000);

        // 验证文件已删除
        const fileItemsAfter = page.locator(
            "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
        );
        let finalCount = await fileItemsAfter.count();
        console.log(`✓ 删除后文件数：${finalCount}`);
        expect(finalCount).toBeLessThan(initialCount);
    });

    test("文件选择和批量操作", async ({ page }) => {
        console.log("\n========== 文件选择测试 ==========");
        await page.goto(TEST_ROOM_URL);
        await page.waitForLoadState("networkidle", { timeout: 15000 }).catch(
            () => {},
        );

        // 等待页面完全加载
        await page.waitForTimeout(1000);

        // 获取文件列表
        const fileItems = page.locator(
            "div.group.relative.flex.items-center.gap-3.rounded-lg.border",
        );
        const itemCount = await fileItems.count();
        console.log(`✓ 文件总数：${itemCount}`);

        if (itemCount > 0) {
            const firstItem = fileItems.first();
            const checkbox = firstItem.locator("[role='checkbox']");
            const isChecked = await checkbox.isChecked();
            console.log(`✓ 文件复选框状态：${isChecked ? "已选" : "未选"}`);

            if (!isChecked) {
                await checkbox.click();
                const afterClick = await checkbox.isChecked();
                console.log(`✓ 点击后状态：${afterClick ? "已选" : "未选"}`);
                expect(afterClick).toBe(true);
            }
        }
    });
});
