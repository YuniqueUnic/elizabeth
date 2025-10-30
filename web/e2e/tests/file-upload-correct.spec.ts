import { test, expect } from "@playwright/test";
import * as fs from "fs";

const BASE_URL = "http://localhost:4001";
const ROOM = "correct-upload-" + Date.now();

test("正确的文件上传测试", async ({ page }) => {
    const logs: string[] = [];

    page.on("console", (msg) => {
        const text = msg.text();
        if (text.includes("[uploadFile]") || text.includes("[getFilesList]")) {
            logs.push(text);
        }
    });

    console.log("第 1 步：打开房间");
    await page.goto(`${BASE_URL}/${ROOM}`);
    await page.waitForLoadState("networkidle").catch(() => {});
    await page.waitForTimeout(2000);

    console.log("第 2 步：找到文件 input");
    const fileInput = page.locator("input[type='file']");

    // 创建测试文件
    const testFile = "/tmp/correct-upload-test.txt";
    fs.writeFileSync(testFile, "test content for correct upload");

    console.log("第 3 步：设置文件并触发 change 事件");
    await fileInput.setInputFiles(testFile);

    // 手动触发 change 事件
    await page.evaluate(() => {
        const input = document.querySelector("input[type='file']") as HTMLInputElement;
        if (input) {
            input.dispatchEvent(new Event("change", { bubbles: true }));
        }
    });

    console.log("第 4 步：等待上传完成");
    await page.waitForTimeout(3000);

    console.log("\n========== 收集的日志 ==========");
    logs.forEach(log => console.log(log));

    console.log("\n第 5 步：验证文件是否出现");
    const fileItems = page.locator("div.group.relative.flex.items-center.gap-3.rounded-lg.border");
    const count = await fileItems.count();
    console.log(`文件列表中的项数：${count}`);

    if (count > 0) {
        console.log("✅ 成功！文件出现在列表中");
    } else {
        console.log("❌ 失败！文件没有出现");
    }
});
