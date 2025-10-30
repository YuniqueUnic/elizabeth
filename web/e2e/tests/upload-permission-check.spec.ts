import { test } from "@playwright/test";
import * as fs from "fs";

const BASE_URL = "http://localhost:4001";
const ROOM = "perm-check-" + Date.now();

test("检查上传权限和 UI", async ({ page }) => {
    page.on("console", (msg) => {
        if (msg.text().includes("[") || msg.text().includes("upload") || msg.text().includes("Upload")) {
            console.log(`[CONSOLE] ${msg.text()}`);
        }
    });

    await page.goto(`${BASE_URL}/${ROOM}`);
    await page.waitForLoadState("networkidle").catch(() => {});
    await page.waitForTimeout(2000);

    // 检查 FileUploadZone 是否存在
    const uploadZone = page.locator('[role="generic"]').filter({ has: page.locator('text=拖拽文件') });
    const uploadZoneCount = await uploadZone.count();
    console.log(`上传区域数：${uploadZoneCount}`);

    // 检查 file input
    const fileInput = page.locator("input[type='file']");
    const fileInputCount = await fileInput.count();
    console.log(`文件 input 数：${fileInputCount}`);

    if (fileInputCount === 0) {
        console.log("❌ 没有找到文件 input - 可能没有编辑权限");
        return;
    }

    console.log("✅ 找到文件 input");

    // 尝试查找拖拽区域
    const dropZone = page.locator("div").filter({ has: page.locator("svg") }).filter({ hasText: /拖拽|上传/ });
    const dropZoneCount = await dropZone.count();
    console.log(`拖拽区域数：${dropZoneCount}`);

    // 打印所有包含"上传"的元素
    const uploadText = page.locator("text=/上传 | 拖拽/");
    const uploadTextCount = await uploadText.count();
    console.log(`包含上传/拖拽文本的元素：${uploadTextCount}`);
});
