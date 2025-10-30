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
            saveBtn: new ButtonElement(
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
        if (permissions.preview !== undefined) {
            const isChecked = await this.roomPermissions.previewBtn
                .getAttribute("aria-pressed");
            if (isChecked === "false" && permissions.preview) {
                await this.roomPermissions.previewBtn.click();
            }
        }

        if (permissions.edit !== undefined) {
            const isChecked = await this.roomPermissions.editBtn.getAttribute(
                "aria-pressed",
            );
            if (isChecked === "false" && permissions.edit) {
                await this.roomPermissions.editBtn.click();
            }
        }

        if (permissions.share !== undefined) {
            const isChecked = await this.roomPermissions.shareBtn.getAttribute(
                "aria-pressed",
            );
            if (isChecked === "false" && permissions.share) {
                await this.roomPermissions.shareBtn.click();
            }
        }

        if (permissions.delete !== undefined) {
            const isChecked = await this.roomPermissions.deleteBtn.getAttribute(
                "aria-pressed",
            );
            if (isChecked === "false" && permissions.delete) {
                await this.roomPermissions.deleteBtn.click();
            }
        }

        await this.roomPermissions.saveBtn.click();
    }

    /**
     * 发送消息
     * 使用示例：await roomPage.sendMessage('Hello World')
     */
    async sendMessage(content: string): Promise<void> {
        await this.messages.input.fill(content);
        await this.messages.sendBtn.click();
        await this.page.waitForTimeout(500);
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
        try {
            // 使用 getByTestId 来获取"未保存"标签（最可靠的方式）
            const unsavedBadges = this.page.getByTestId(
                /^message-unsaved-badge-/,
            );
            // 如果存在至少一个未保存标签，说明有消息未保存
            return (await unsavedBadges.count()) > 0;
        } catch (error) {
            return false;
        }
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
        // 等待左侧边栏和中间列同时出现
        const timeout = 30000; // 30 秒

        try {
            // 等待左侧边栏
            console.log("等待左侧边栏...");
            await this.page.locator(htmlSelectors.leftSidebar.container)
                .waitFor({
                    state: "visible",
                    timeout: timeout / 2,
                });
            console.log("左侧边栏加载成功");

            // 等待中间列
            console.log("等待中间列...");
            await this.page.locator(htmlSelectors.middleColumn.container)
                .waitFor({
                    state: "visible",
                    timeout: timeout / 2,
                });
            console.log("中间列加载成功");

            // 额外等待消息输入框准备就绪
            console.log("等待消息输入框...");
            await this.page.locator(htmlSelectors.middleColumn.editor.input)
                .waitFor({
                    state: "visible",
                    timeout: 5000,
                });
            console.log("消息输入框加载成功");
        } catch (error: any) {
            // 提供更详细的错误信息
            console.error("房间加载失败：", error.message);

            // 检查每个元素的状态
            try {
                const sidebarVisible = await this.page.locator(
                    htmlSelectors.leftSidebar.container,
                ).isVisible();
                console.error("左侧边栏可见：", sidebarVisible);
            } catch (e) {
                console.error("左侧边栏检查失败");
            }

            try {
                const mainVisible = await this.page.locator(
                    htmlSelectors.middleColumn.container,
                ).isVisible();
                console.error("中间列可见：", mainVisible);
            } catch (e) {
                console.error("中间列检查失败");
            }

            const content = await this.page.content();
            console.error(
                "Page content preview:",
                content?.substring(0, 300),
            );
            throw new Error(
                `Room failed to load. Check console logs for details.`,
            );
        }
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
