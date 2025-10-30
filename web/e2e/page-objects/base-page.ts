/**
 * 基础 PageObject 类
 * 提供通用的页面操作方法
 */

import { expect, Locator, Page } from "@playwright/test";
import { htmlSelectors } from "../selectors/html-selectors";

export abstract class BasePage {
    protected _page: Page;
    protected selectors = htmlSelectors;

    constructor(page: Page) {
        this._page = page;
    }

    /**
     * 获取 Page 对象（公共访问）
     */
    get page(): Page {
        return this._page;
    }

    /**
     * 获取页面 URL
     */
    getURL(): string {
        return this._page.url();
    }

    /**
     * 导航到指定 URL
     */
    async goto(url: string): Promise<void> {
        await this._page.goto(url);
    }

    /**
     * 获取页面标题
     */
    async getTitle(): Promise<string> {
        return await this._page.title();
    }

    /**
     * 等待导航完成
     */
    async waitForNavigation(): Promise<void> {
        await this._page.waitForNavigation();
    }

    /**
     * 刷新页面
     */
    async reload(): Promise<void> {
        await this._page.reload();
    }

    /**
     * 返回上一页
     */
    async goBack(): Promise<void> {
        await this._page.goBack();
    }

    /**
     * 前进到下一页
     */
    async goForward(): Promise<void> {
        await this._page.goForward();
    }

    /**
     * 查找元素
     */
    locator(selector: string): Locator {
        return this._page.locator(selector);
    }

    /**
     * 获取元素文本
     */
    async getElementText(selector: string): Promise<string> {
        return await this._page.locator(selector).textContent() || "";
    }

    /**
     * 检查元素是否可见
     */
    async isElementVisible(selector: string): Promise<boolean> {
        try {
            await this._page.locator(selector).waitFor({
                state: "visible",
                timeout: 2000,
            });
            return true;
        } catch {
            return false;
        }
    }

    /**
     * 等待元素出现
     */
    async waitForElement(
        selector: string,
        timeout: number = 5000,
    ): Promise<void> {
        await this._page.locator(selector).waitFor({
            state: "visible",
            timeout,
        });
    }

    /**
     * 等待元素消失
     */
    async waitForElementHidden(
        selector: string,
        timeout: number = 5000,
    ): Promise<void> {
        await this._page.locator(selector).waitFor({
            state: "hidden",
            timeout,
        });
    }

    /**
     * 等待 Toast 通知出现
     */
    async waitForToast(
        title: string | RegExp,
        timeout: number = 5000,
    ): Promise<void> {
        const pattern = typeof title === "string"
            ? `text="${title}"`
            : `text=/${title}/`;
        await this._page.locator(`role=status >> ${pattern}`).waitFor({
            state: "visible",
            timeout,
        });
    }

    /**
     * 等待 Toast 消失
     */
    async waitForToastGone(
        title: string | RegExp,
        timeout: number = 5000,
    ): Promise<void> {
        const pattern = typeof title === "string"
            ? `text="${title}"`
            : `text=/${title}/`;
        await this._page.locator(`role=status >> ${pattern}`).waitFor({
            state: "hidden",
            timeout,
        });
    }

    /**
     * 获取 Toast 消息
     */
    async getToastMessage(): Promise<string> {
        const toast = this._page.locator("role=status").first();
        return await toast.textContent() || "";
    }

    /**
     * 填充输入框
     */
    async fillInput(selector: string, text: string): Promise<void> {
        await this._page.locator(selector).fill(text);
    }

    /**
     * 点击按钮
     */
    async clickButton(selector: string): Promise<void> {
        await this._page.locator(selector).click();
    }

    /**
     * 选择下拉框选项
     */
    async selectOption(selector: string, option: string): Promise<void> {
        await this._page.locator(selector).selectOption(option);
    }

    /**
     * 检查复选框
     */
    async checkCheckbox(selector: string): Promise<void> {
        await this._page.locator(selector).check();
    }

    /**
     * 取消检查复选框
     */
    async uncheckCheckbox(selector: string): Promise<void> {
        await this._page.locator(selector).uncheck();
    }

    /**
     * 等待加载指示器消失
     */
    async waitForLoadingComplete(timeout: number = 10000): Promise<void> {
        const loader = this._page.locator(
            '[aria-label="loading"], .spinner, .loader',
        );
        try {
            await loader.waitFor({ state: "hidden", timeout });
        } catch {
            // 加载指示器可能不存在，这是正常的
        }
    }

    /**
     * 截图
     */
    async screenshot(name: string): Promise<void> {
        await this._page.screenshot({ path: `./screenshots/${name}.png` });
    }

    /**
     * 打印 HTML 快照用于调试
     */
    async printSnapshot(): Promise<void> {
        const html = await this._page.content();
        console.log(html);
    }

    /**
     * 获取选择器对象（用于链式调用）
     */
    getSelectors() {
        return this.selectors;
    }

    /**
     * 获取页面对象本身（用于链式调用）
     */
    getPage(): Page {
        return this._page;
    }
}
