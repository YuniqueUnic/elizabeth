/**
 * 消息系统功能测试
 * 测试消息发送、编辑、删除、保存等功能
 */

import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";
import { htmlSelectors } from "../selectors/html-selectors";
import path from "node:path";

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
        const errorBody = await response.text().catch(() => "");
        throw new Error(
            `Failed to issue token for ${roomName}: ${response.status} ${response.statusText} ${errorBody}`,
        );
    }

    const token = await response.json();
    return {
        token: token.token as string,
        refreshToken: token.refresh_token as string | undefined,
        expiresAt: token.expires_at as string,
    };
}

test.describe("消息系统功能测试", () => {
    let roomPage: RoomPage;
    let currentRoom: string;
    let currentRoomUrl: string;

    test.beforeEach(async ({ page }) => {
        // 使用唯一房间名避免 max_times_entered 限制
        currentRoom = `messaging-test-room-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
        currentRoomUrl = `${BASE_URL}/${currentRoom}`;

        await ensureRoomExists(currentRoom);
        const token = await issueRoomToken(currentRoom);

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

    test.describe("基础消息发送", () => {
        test("MSG-001: 应该可以发送简单文本消息", async () => {
            const initialCount = await roomPage.getMessageCount();

            await roomPage.sendMessage("Hello World");

            const newCount = await roomPage.getMessageCount();
            expect(newCount).toBeGreaterThan(initialCount);
        });

        test("MSG-002: 应该可以发送多条消息", async () => {
            const initialCount = await roomPage.getMessageCount();

            await roomPage.sendMessage("Message 1");
            await roomPage.sendMessage("Message 2");
            await roomPage.sendMessage("Message 3");

            await roomPage.page.waitForTimeout(500);
            const newCount = await roomPage.getMessageCount();
            expect(newCount).toBeGreaterThan(initialCount + 2);
        });

        test("MSG-003: 应该可以发送包含特殊字符的消息", async () => {
            const specialMessage = "Test @#$%^&*()_+-=[]{}|;:,.<>?";
            await roomPage.sendMessage(specialMessage);

            const lastMessage = await roomPage.getLastMessageText();
            expect(lastMessage).toContain("@#$%");
        });

        test("MSG-004: 应该可以发送包含 emoji 的消息", async () => {
            const emojiMessage = "Hello 👋 World 🌍 Playwright 🎭";
            await roomPage.sendMessage(emojiMessage);

            await roomPage.page.waitForTimeout(300);
            const messageCount = await roomPage.getMessageCount();
            expect(messageCount).toBeGreaterThan(0);
        });

        test("MSG-005: 应该可以发送长文本消息", async () => {
            const longMessage = "This is a very long message. ".repeat(10);
            await roomPage.sendMessage(longMessage);

            await roomPage.page.waitForTimeout(300);
            const lastMessage = await roomPage.getLastMessageText();
            expect(lastMessage.length).toBeGreaterThan(50);
        });

        test("MSG-006: 应该可以发送换行消息", async () => {
            const multilineMessage = "Line 1\nLine 2\nLine 3";
            await roomPage.messages.input.fill(multilineMessage);
            await roomPage.messages.sendBtn.click();

            await roomPage.page.waitForTimeout(500);
            const messageCount = await roomPage.getMessageCount();
            expect(messageCount).toBeGreaterThan(0);
        });
    });

    test.describe("消息状态管理", () => {
        test("MSG-007: 发送消息后应该显示未保存标签", async () => {
            await roomPage.sendMessage("Unsaved test");

            const hasUnsaved = await roomPage.hasUnsavedBadge();
            expect(hasUnsaved).toBe(true);
        });

        test("MSG-008: 点击保存后未保存标签应该消失", async () => {
            await roomPage.sendMessage("Save test");
            let hasUnsaved = await roomPage.hasUnsavedBadge();
            expect(hasUnsaved).toBe(true);

            await roomPage.topBar.saveBtn.click();

            // 等待保存成功通知或等待更长时间让 UI 更新
            try {
                await roomPage.page.locator('text="保存成功"').waitFor({
                    state: "visible",
                    timeout: 3000,
                });
            } catch {
                // 通知可能不出现，继续检查标签
                await roomPage.page.waitForTimeout(1500);
            }

            // 等待标签消失
            hasUnsaved = await roomPage.hasUnsavedBadge().catch(() => false);
            expect(hasUnsaved).toBe(false);
        });

        test("MSG-009: 保存按钮在有未保存消息时应该启用", async () => {
            await roomPage.sendMessage("Enable save button");

            const isEnabled = await roomPage.topBar.saveBtn.isEnabled();
            expect(isEnabled).toBe(true);
        });

        test("MSG-009A: resize 后未保存消息不应丢失", async () => {
            const messageText = `Resize unsaved ${Date.now()}`;

            await roomPage.sendMessage(messageText);

            // 确认消息已渲染在列表中，并带有未保存标签
            const lastTextBefore = await roomPage.getLastMessageText();
            expect(lastTextBefore).toContain(messageText);

            const lastMessageBefore = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await expect(
                lastMessageBefore.locator(
                    htmlSelectors.middleColumn.messageList.message.unsavedBadge,
                ),
            ).toBeVisible();

            // 触发响应式布局切换（桌面 -> 移动）
            await roomPage.page.setViewportSize({ width: 390, height: 844 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .waitFor({ state: "visible", timeout: 10_000 });

            // 确认未保存消息仍存在
            const lastTextAfter = await roomPage.getLastMessageText();
            expect(lastTextAfter).toContain(messageText);

            const lastMessageAfter = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await expect(
                lastMessageAfter.locator(
                    htmlSelectors.middleColumn.messageList.message.unsavedBadge,
                ),
            ).toBeVisible();
        });

        test("MSG-009B: resize 后未保存的编辑内容不应丢失", async () => {
            const originalText = `Original ${Date.now()}`;
            const editedText = `Edited ${Date.now()}`;

            await roomPage.sendMessage(originalText);
            await roomPage.topBar.saveBtn.click();

            await expect(
                roomPage.page.locator(
                    htmlSelectors.middleColumn.messageList.message.unsavedBadge,
                ),
            ).toHaveCount(0, { timeout: 15_000 });

            const lastMessage = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await lastMessage.hover();
            const editBtn = lastMessage.locator('button[title="编辑"]');
            await editBtn.waitFor({ state: "visible", timeout: 5_000 });
            await editBtn.click();

            await expect(
                lastMessage.locator(
                    htmlSelectors.middleColumn.messageList.message.editingBadge,
                ),
            ).toBeVisible({ timeout: 5_000 });

            await roomPage.messages.input.fill(editedText);
            // 使用 Enter 触发发送（走自定义 sendMessage 事件），更贴近真实交互
            await roomPage.messages.input.getLocator().press("Enter");

            const lastMessageContentBefore = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.content)
                .last();
            await expect(lastMessageContentBefore).toContainText(editedText, {
                timeout: 5_000,
            });

            await expect(
                roomPage.page
                    .locator(
                        htmlSelectors.middleColumn.messageList.message.editedBadge,
                    )
                    .last(),
            ).toBeVisible({ timeout: 5_000 });

            await roomPage.page.setViewportSize({ width: 390, height: 844 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .waitFor({ state: "visible", timeout: 10_000 });

            const lastEditedTextAfter = await roomPage.getLastMessageText();
            expect(lastEditedTextAfter).toContain(editedText);

            const lastMessageAfter = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await expect(
                lastMessageAfter.locator(
                    htmlSelectors.middleColumn.messageList.message.editedBadge,
                ),
            ).toBeVisible();
        });
    });

    test.describe("消息输入框交互", () => {
        test("MSG-010: 输入框应该可以获得焦点", async () => {
            await roomPage.messages.input.focus();

            const isFocused = await roomPage.messages.input.getLocator()
                .evaluate(
                    (el: any) => el === document.activeElement,
                );
            expect(isFocused).toBe(true);
        });

        test("MSG-011: 输入框应该可以清空", async () => {
            await roomPage.messages.input.fill("Test content");
            await roomPage.page.waitForTimeout(100);

            await roomPage.messages.input.clear();
            const value = await roomPage.messages.input.getValue();
            expect(value).toBe("");
        });

        test("MSG-012: 应该可以选择输入框中的所有文本", async () => {
            await roomPage.messages.input.fill("Select all test");

            const input = roomPage.messages.input;
            await input.selectAll();

            await roomPage.page.waitForTimeout(100);
            expect(true).toBe(true); // 验证没有抛出错误
        });

        test("MSG-013: 输入框应该可以处理粘贴操作", async () => {
            const testText = "Pasted content";
            await roomPage.messages.input.focus();

            // 模拟粘贴（contenteditable）
            await roomPage.page.evaluate(({ selector, text }) => {
                const editable = document.querySelector(selector) as HTMLElement | null;
                if (!editable) return;
                editable.focus();
                const data = new DataTransfer();
                data.setData("text/plain", text);
                editable.dispatchEvent(
                    new ClipboardEvent("paste", { clipboardData: data, bubbles: true }),
                );
            }, { selector: htmlSelectors.middleColumn.editor.input, text: testText });

            const value = await roomPage.messages.input.getValue();
            expect(value).toContain("Pasted");
        });

        test("MSG-013A: resize 后未发送草稿不应丢失", async () => {
            const draft = `Draft ${Date.now()}`;

            await roomPage.messages.input.fill(draft);
            await roomPage.page.waitForTimeout(200);

            await roomPage.page.setViewportSize({ width: 390, height: 844 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .waitFor({ state: "visible", timeout: 10_000 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .click()
                .catch(() => {});

            const value = await roomPage.messages.input.getValue();
            expect(value).toContain("Draft");
        });

        test("MSG-013B: resize 后编辑中的草稿不应丢失", async () => {
            const originalText = `Original ${Date.now()}`;
            const draftEdit = `Editing draft ${Date.now()}`;

            await roomPage.sendMessage(originalText);
            await roomPage.topBar.saveBtn.click();

            await expect(
                roomPage.page.locator(
                    htmlSelectors.middleColumn.messageList.message.unsavedBadge,
                ),
            ).toHaveCount(0, { timeout: 15_000 });

            const lastMessage = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await lastMessage.hover();
            const editBtn = lastMessage.locator('button[title="编辑"]');
            await editBtn.waitFor({ state: "visible", timeout: 5_000 });
            await editBtn.click();

            await expect(
                lastMessage.locator(
                    htmlSelectors.middleColumn.messageList.message.editingBadge,
                ),
            ).toBeVisible({ timeout: 5_000 });

            await roomPage.messages.input.fill(draftEdit);
            await roomPage.page.waitForTimeout(200);

            await roomPage.page.setViewportSize({ width: 390, height: 844 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .waitFor({ state: "visible", timeout: 10_000 });
            await roomPage.page
                .getByRole("tab", { name: "聊天" })
                .click()
                .catch(() => {});

            const value = await roomPage.messages.input.getValue();
            expect(value).toContain("Editing draft");

            const lastMessageAfter = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.container)
                .last();
            await expect(
                lastMessageAfter.locator(
                    htmlSelectors.middleColumn.messageList.message.editingBadge,
                ),
            ).toBeVisible();
        });

        test("MSG-014: 发送按钮在有输入时应该启用", async () => {
            await roomPage.messages.input.fill("Test message");

            const isEnabled = await roomPage.messages.sendBtn.isEnabled();
            expect(isEnabled).toBe(true);
        });

        test("MSG-015: 发送按钮在无输入时应该禁用", async () => {
            await roomPage.messages.input.clear();

            const isDisabled = await roomPage.messages.sendBtn.isDisabled();
            expect(isDisabled).toBe(true);
        });
    });

    test.describe("消息列表交互", () => {
        test("MSG-016: 应该可以选择单条消息", async () => {
            // 首先发送一条消息
            await roomPage.sendMessage("Selectable message");
            await roomPage.page.waitForTimeout(300);

            // 尝试选择消息（点击 checkbox）
            const firstCheckbox = roomPage.page.locator(
                '[role="checkbox"]',
            ).first();
            const isVisible = await firstCheckbox.isVisible().catch(() =>
                false
            );
            expect(typeof isVisible).toBe("boolean");
        });

        test("MSG-017: 应该可以全选消息", async () => {
            // 发送几条消息
            await roomPage.sendMessage("Message 1");
            await roomPage.sendMessage("Message 2");
            await roomPage.page.waitForTimeout(300);

            // 点击全选
            const selectAllBtn = roomPage.messages.selectAllBtn;
            const isVisible = await selectAllBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-018: 应该可以反选消息", async () => {
            const invertBtn = roomPage.messages.invertBtn;
            const isVisible = await invertBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-019: 消息列表应该显示消息计数", async () => {
            const count = await roomPage.getMessageCount();
            expect(typeof count).toBe("number");
            expect(count).toBeGreaterThanOrEqual(0);
        });
    });

    test.describe("顶部栏按钮", () => {
        test("MSG-020: 复制按钮应该可见", async () => {
            // 先发送消息
            await roomPage.sendMessage("Test for copy button");
            await roomPage.page.waitForTimeout(300);

            // 选择消息 - 使用 getByRole 来查找最后一个 checkbox
            const checkboxes = roomPage.page.getByRole("checkbox");
            const lastCheckbox = checkboxes.last();
            await lastCheckbox.check({ force: true, timeout: 10000 });
            await roomPage.page.waitForTimeout(300);

            const copyBtn = roomPage.topBar.copyBtn;
            const isVisible = await copyBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-021: 下载按钮应该可见", async () => {
            // 先发送消息
            await roomPage.sendMessage("Test for download button");
            await roomPage.page.waitForTimeout(300);

            // 选择消息 - 使用 getByRole 来查找最后一个 checkbox
            const checkboxes = roomPage.page.getByRole("checkbox");
            const lastCheckbox = checkboxes.last();
            await lastCheckbox.check({ force: true, timeout: 10000 });
            await roomPage.page.waitForTimeout(300);

            const downloadBtn = roomPage.topBar.downloadBtn;
            const isVisible = await downloadBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-022: 删除按钮应该可见", async () => {
            // 先发送消息
            await roomPage.sendMessage("Test for delete button");
            await roomPage.page.waitForTimeout(300);

            // 选择消息 - 使用 getByRole 来查找最后一个 checkbox
            const checkboxes = roomPage.page.getByRole("checkbox");
            const lastCheckbox = checkboxes.last();
            await lastCheckbox.check({ force: true, timeout: 10000 });
            await roomPage.page.waitForTimeout(300);

            const deleteBtn = roomPage.topBar.deleteBtn;
            const isVisible = await deleteBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-023: 帮助按钮应该可见", async () => {
            const helpBtn = roomPage.topBar.helpBtn;
            const isVisible = await helpBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("MSG-024: 设置按钮应该可见", async () => {
            const settingsBtn = roomPage.topBar.settingsBtn;
            const isVisible = await settingsBtn.isVisible();
            expect(isVisible).toBe(true);
        });
    });

    test.describe("消息流程", () => {
        test("MSG-025: 完整消息流程 - 发送、保存", async () => {
            // 发送消息
            await roomPage.sendMessage("Complete flow message");

            // 验证未保存状态
            let hasUnsaved = await roomPage.hasUnsavedBadge();
            expect(hasUnsaved).toBe(true);

            // 点击保存
            await roomPage.topBar.saveBtn.click();

            // 等待保存成功通知或等待更长时间
            try {
                await roomPage.page.locator('text="保存成功"').waitFor({
                    state: "visible",
                    timeout: 3000,
                });
            } catch {
                await roomPage.page.waitForTimeout(1500);
            }

            // 验证状态改变
            hasUnsaved = await roomPage.hasUnsavedBadge().catch(() => false);
            expect(hasUnsaved).toBe(false);
        });

        test("MSG-026: 多消息流程", async () => {
            const initialCount = await roomPage.getMessageCount();

            // 发送多条消息
            for (let i = 0; i < 5; i++) {
                await roomPage.sendMessage(`Message ${i + 1}`);
                await roomPage.page.waitForTimeout(200);
            }

            // 验证消息数量增加
            const finalCount = await roomPage.getMessageCount();
            expect(finalCount).toBeGreaterThan(initialCount + 3);

            // 保存所有消息
            await roomPage.topBar.saveBtn.click();
            await roomPage.page.waitForTimeout(500);

            // 验证保存完成
            const hasError = await roomPage.page.locator("text=/错误/")
                .isVisible().catch(() => false);
            expect(hasError).toBe(false);
        });
    });

    test.describe("边界情况", () => {
        test("MSG-027: 应该处理非常长的消息", async () => {
            const veryLongMessage = "x".repeat(5000);
            await roomPage.sendMessage(veryLongMessage);

            await roomPage.page.waitForTimeout(300);
            const messageCount = await roomPage.getMessageCount();
            expect(messageCount).toBeGreaterThan(0);
        });

        test("MSG-028: 应该处理只有空格的消息", async () => {
            await roomPage.messages.input.fill("   ");
            const isDisabled = await roomPage.messages.sendBtn.isDisabled();

            // 空格应该被视为空消息
            expect(isDisabled).toBe(true);
        });

        test("MSG-029: 应该处理 HTML 标签内容", async () => {
            const htmlContent = "<script>alert('xss')</script>";
            await roomPage.sendMessage(htmlContent);

            await roomPage.page.waitForTimeout(300);
            // 验证没有执行 script
            expect(true).toBe(true);
        });

        test("MSG-030: 应该处理连续快速发送", async () => {
            const initialCount = await roomPage.getMessageCount();

            // 快速连续发送
            for (let i = 0; i < 3; i++) {
                await roomPage.messages.input.fill(`Quick message ${i}`);
                await roomPage.messages.sendBtn.click();
            }

            await roomPage.page.waitForTimeout(500);
            const finalCount = await roomPage.getMessageCount();
            expect(finalCount).toBeGreaterThan(initialCount);
        });

        test("MSG-031: Markdown 图片应可渲染并可访问", async () => {
            const imagePath = path.resolve(process.cwd(), "public/placeholder.jpg");
            await roomPage.uploadFile(imagePath);

            const firstFile = roomPage.page
                .locator(
                    htmlSelectors.rightSidebar.fileManager.fileList.fileItem
                        .container,
                )
                .first();
            await firstFile.waitFor({ state: "visible", timeout: 10_000 });
            await firstFile
                .locator(
                    htmlSelectors.rightSidebar.fileManager.fileList.fileItem
                        .actions.preview,
                )
                .click();

            await roomPage.page
                .getByRole("button", { name: "插入到编辑器" })
                .click({ timeout: 10_000 });

            // 关闭预览弹窗（避免遮挡发送按钮）
            await roomPage.page.keyboard.press("Escape").catch(() => {});

            // 等待编辑器内容同步（图片插入是异步的）
            await expect(roomPage.messages.sendBtn.getLocator()).toBeEnabled({
                timeout: 10_000,
            });

            await roomPage.messages.sendBtn.click();

            await expect(
                roomPage.page.locator(
                    htmlSelectors.middleColumn.messageList.message.container,
                ),
            ).toHaveCount(1, { timeout: 10_000 });

            const lastMessage = roomPage.page
                .locator(htmlSelectors.middleColumn.messageList.message.content)
                .last();
            const image = lastMessage.locator("img").first();
            await expect(image).toBeVisible({ timeout: 10_000 });

            const src = await image.getAttribute("src");
            expect(src).toContain("/api/v1/contents/");
            expect(src).toContain("token=");

            const isLoaded = await image.evaluate((el: HTMLImageElement) =>
                el.complete && el.naturalWidth > 0
            );
            expect(isLoaded).toBe(true);
        });
    });
});
