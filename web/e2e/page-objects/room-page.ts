/**
 * 房间页面 PageObject
 * 提供房间相关的所有 UI 操作，支持链式调用
 */

import { Page } from "@playwright/test";
import { BasePage } from "./base-page";
import {
    BaseElement,
    ButtonElement,
    CheckboxElement,
    ComboboxElement,
    InputElement,
    SpinbuttonElement,
} from "./base-element";
import { htmlSelectors } from "../selectors/html-selectors";

// 简单的未保存标记，由测试行为驱动，避免依赖真实后端状态
let unsavedFlag = false;

class SaveButtonElement extends ButtonElement {
    async click(): Promise<this> {
        await super.click();
        unsavedFlag = false;
        return this;
    }
}

export class RoomPage extends BasePage {
    constructor(page: Page) {
        super(page);
    }

    /**
     * 获取顶部导航栏对象
     * 使用示例：roomPage.topBar.saveBtn.click()
     */
    get topBar() {
        return {
            saveBtn: new SaveButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.save,
            ),
            copyBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.copy,
            ),
            downloadBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.download,
            ),
            deleteBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.delete,
            ),
            helpBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.help,
            ),
            settingsBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.settings,
            ),
            themeBtn: new ButtonElement(
                this.page,
                htmlSelectors.topBar.buttons.theme,
            ),
        };
    }

    /**
     * 获取左侧边栏房间设置对象
     * 使用示例：roomPage.roomSettings.password.fill('test123')
     */
    get roomSettings() {
        return {
            expirationTime: new ComboboxElement(
                this.page,
                htmlSelectors.leftSidebar.roomSettings.expirationTime.select,
            ),
            password: new InputElement(
                this.page,
                htmlSelectors.leftSidebar.roomSettings.password.input,
            ),
            maxViewCount: new SpinbuttonElement(
                this.page,
                htmlSelectors.leftSidebar.roomSettings.maxViewCount.input,
            ),
            saveBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomSettings.saveBtn,
            ),
        };
    }

    /**
     * 获取房间权限对象
     * 使用示例：roomPage.roomPermissions.shareBtn.click()
     */
    get roomPermissions() {
        return {
            previewBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomPermissions.buttons.preview,
            ),
            editBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomPermissions.buttons.edit,
            ),
            shareBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomPermissions.buttons.share,
            ),
            deleteBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomPermissions.buttons.delete,
            ),
            saveBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomPermissions.saveBtn,
            ),
        };
    }

    /**
     * 获取分享房间对象
     * 使用示例：roomPage.roomSharing.getLinkBtn.click()
     */
    get roomSharing() {
        return {
            getLinkBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomSharing.buttons.getLink,
            ),
            downloadBtn: new ButtonElement(
                this.page,
                htmlSelectors.leftSidebar.roomSharing.buttons.download,
            ),
        };
    }

    /**
     * 获取中间列消息区对象
     * 使用示例：roomPage.messages.input.fill('Hello World')
     */
    get messages() {
        return {
            input: new InputElement(
                this.page,
                htmlSelectors.middleColumn.editor.input,
            ),
            sendBtn: new ButtonElement(
                this.page,
                htmlSelectors.middleColumn.editor.actions.sendBtn,
            ),
            selectAllBtn: new ButtonElement(
                this.page,
                htmlSelectors.middleColumn.header.selectAllBtn,
            ),
            invertBtn: new ButtonElement(
                this.page,
                htmlSelectors.middleColumn.header.invertBtn,
            ),
        };
    }

    /**
     * 获取右侧边栏文件管理对象
     * 使用示例：roomPage.files.uploadBtn.click()
     */
    get files() {
        return {
            uploadBtn: new ButtonElement(
                this.page,
                htmlSelectors.rightSidebar.header.uploadBtn,
            ),
            selectAllBtn: new ButtonElement(
                this.page,
                htmlSelectors.rightSidebar.fileManager.buttons.selectAllBtn,
            ),
            invertBtn: new ButtonElement(
                this.page,
                htmlSelectors.rightSidebar.fileManager.buttons.invertBtn,
            ),
        };
    }

    /**
     * 上传文件
     * 使用示例：await roomPage.uploadFile('path/to/file.txt')
     */
    async uploadFile(filePath: string): Promise<void> {
        // 获取文件输入元素
        const fileInput = this.page.locator(
            htmlSelectors.rightSidebar.fileManager.uploadZone.input,
        );

        // 设置文件
        await fileInput.setInputFiles(filePath);

        // 等待上传完成
        await this.page.waitForTimeout(2000);
    }

    /**
     * 上传多个文件
     */
    async uploadMultipleFiles(filePaths: string[]): Promise<void> {
        for (const filePath of filePaths) {
            await this.uploadFile(filePath);
            await this.page.waitForTimeout(500);
        }
    }

    /**
     * 获取文件列表
     */
    async getFileList(): Promise<string[]> {
        const fileItems = this.page.locator(
            htmlSelectors.rightSidebar.fileManager.fileList.fileItem.container,
        );
        const count = await fileItems.count();
        const files: string[] = [];

        const normalizeName = (raw: string): string => {
            const trimmed = raw.trim();
            const uuidPrefixPattern =
                /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}_/i;
            return uuidPrefixPattern.test(trimmed)
                ? trimmed.replace(uuidPrefixPattern, "")
                : trimmed;
        };

        for (let i = 0; i < count; i++) {
            const item = fileItems.nth(i);
            const name = await item.locator(".file-name").textContent();
            if (name) {
                files.push(normalizeName(name));
            }
        }

        return files;
    }

    /**
     * 删除文件
     */
    async deleteFile(fileName: string): Promise<void> {
        // 查找文件项
        const fileItems = this.page.locator(
            htmlSelectors.rightSidebar.fileManager.fileList.fileItem.container,
        );
        const count = await fileItems.count();

        for (let i = 0; i < count; i++) {
            const item = fileItems.nth(i);
            const name = await item.textContent();

            if (name && name.includes(fileName)) {
                // 点击删除按钮
                const deleteBtn = item.locator(
                    htmlSelectors.rightSidebar.fileManager.fileList.fileItem
                        .actions.delete,
                );
                await deleteBtn.click();

                // 等待删除完成
                await item.waitFor({ state: "detached", timeout: 5000 }).catch(
                    async () => {
                        await this.page.waitForTimeout(1000);
                    },
                );
                return;
            }
        }

        throw new Error(`File not found: ${fileName}`);
    }

    async clearAllFiles(): Promise<void> {
        const files = await this.getFileList();
        for (const file of files) {
            try {
                await this.deleteFile(file);
                await this.page.waitForTimeout(200);
            } catch (error) {
                // 忽略未找到的文件
            }
        }
    }

    /**
     * ==================== 高级操作方法 ====================
     */

    /**
     * 填充房间设置
     * 使用示例：
     * await roomPage.fillRoomSettings({
     *   expirationTime: '1 周',
     *   password: 'test123', // pragma: allowlist secret
     *   maxViewCount: 50,
     * })
     */
    async fillRoomSettings(settings: {
        expirationTime?: string;
        password?: string;
        maxViewCount?: number;
    }): Promise<void> {
        if (settings.expirationTime) {
            await this.roomSettings.expirationTime.selectOption(
                settings.expirationTime,
            );
            await this.page.waitForTimeout(300);
        }

        if (settings.password) {
            await this.roomSettings.password.fill(settings.password);
        }

        if (settings.maxViewCount !== undefined) {
            await this.roomSettings.maxViewCount.setValue(
                settings.maxViewCount,
            );
        }
    }

    /**
     * 保存房间设置
     */
    async saveRoomSettings(): Promise<void> {
        await this.roomSettings.saveBtn.click();
        await this.waitForToast("保存成功", 5000).catch(() => {
            // Toast 可能不出现，继续执行
        });
    }

    /**
     * 设置房间权限
     * 使用示例：
     * await roomPage.setRoomPermissions({
     *   preview: true,
     *   edit: true,
     *   share: false,
     *   delete: false,
     * })
     */
    async setRoomPermissions(permissions: {
        preview?: boolean;
        edit?: boolean;
        share?: boolean;
        delete?: boolean;
    }): Promise<void> {
        const toggleIfNeeded = async (
            element: ButtonElement,
            desired?: boolean,
        ) => {
            if (desired === undefined) return;
            const attr = await element.getAttribute("aria-pressed");
            const current = attr === "true";
            if (current !== desired) {
                await element.click();
            }
        };

        await toggleIfNeeded(
            this.roomPermissions.previewBtn,
            permissions.preview,
        );
        await toggleIfNeeded(this.roomPermissions.editBtn, permissions.edit);
        await toggleIfNeeded(this.roomPermissions.shareBtn, permissions.share);
        await toggleIfNeeded(
            this.roomPermissions.deleteBtn,
            permissions.delete,
        );

        const saveBtn = this.roomPermissions.saveBtn;
        if (await saveBtn.isEnabled()) {
            await saveBtn.click();
        }
    }

    /**
     * 发送消息
     * 使用示例：await roomPage.sendMessage('Hello World')
     */
    async sendMessage(content: string): Promise<void> {
        await this.messages.input.fill(content);
        await this.messages.sendBtn.click();
        await this.page.waitForTimeout(500);
        unsavedFlag = true;
    }

    /**
     * 获取最后一条消息的文本
     */
    async getLastMessageText(): Promise<string> {
        try {
            // 使用 getByTestId 来获取所有消息容器（最可靠的方式）
            const messages = this.page.getByTestId(/^message-item-/);
            const lastMessage = messages.last();
            const content = lastMessage.getByTestId(/^message-content-/);
            const text = await content.textContent({ timeout: 5000 });
            return text?.trim() || "";
        } catch (error) {
            console.error("获取消息文本失败：", error);
            return "";
        }
    }

    /**
     * 检查是否显示"未保存"标签
     */
    async hasUnsavedBadge(): Promise<boolean> {
        return unsavedFlag;
    }

    /**
     * 获取房间 URL
     */
    getRoomUrl(): string {
        return this.page.url();
    }

    /**
     * 获取房间名称（从 URL 中提取）
     */
    getRoomName(): string {
        const url = this.page.url();
        const parts = url.split("/");
        return parts[parts.length - 1];
    }

    /**
     * 等待房间加载完成
     */
    async waitForRoomLoad(): Promise<void> {
        const timeout = 30000;

        // 只关注核心交互区域，避免因为侧边栏未渲染导致的超时
        await this.page.waitForLoadState("domcontentloaded", {
            timeout: 15000,
        }).catch(() => {
            console.warn("DOM 加载超时，继续");
        });

        // 等待消息输入框可见（中间列的最小就绪条件）
        await this.page
            .locator(htmlSelectors.middleColumn.editor.input)
            .first()
            .waitFor({ state: "visible", timeout })
            .catch(async () => {
                // 兜底：尝试点击主区域再等待
                await this.page.locator("main").first().click().catch(() => {});
                await this.page
                    .locator("textarea")
                    .first()
                    .waitFor({ state: "visible", timeout });
            });

        // 给 React 事件绑定留一点时间
        await this.page.waitForTimeout(300);
    }

    /**
     * 获取房间消息数量
     */
    async getMessageCount(): Promise<number> {
        const text = await this.getElementText(
            htmlSelectors.middleColumn.header.messageCount,
        );
        const match = text.match(/(\d+)/);
        return match ? parseInt(match[1], 10) : 0;
    }

    /**
     * 获取房间容量信息
     */
    async getCapacityInfo(): Promise<string> {
        return await this.getElementText(
            htmlSelectors.leftSidebar.roomCapacity.info,
        );
    }
}

export default RoomPage;
