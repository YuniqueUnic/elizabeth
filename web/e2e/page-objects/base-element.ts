/**
 * 基础元素类
 * 提供通用的元素操作，支持链式调用（fluent interface）
 */

import { expect, Locator, Page } from "@playwright/test";

export interface ElementOptions {
    timeout?: number;
    waitForElement?: boolean;
}

type ResolvedElementOptions = {
    timeout: number;
    waitForElement: boolean;
};

export class BaseElement {
    protected page: Page;
    protected selector: string;
    protected locator: Locator;
    protected options: ResolvedElementOptions;

    constructor(page: Page, selector: string, options: ElementOptions = {}) {
        this.page = page;
        this.selector = selector;
        this.locator = page.locator(selector);
        this.options = {
            timeout: 5000,
            waitForElement: true,
            ...options,
        };
    }

    /**
     * 获取 Playwright Locator 对象
     */
    getLocator(): Locator {
        return this.locator;
    }

    /**
     * 获取选择器
     */
    getSelector(): string {
        return this.selector;
    }

    /**
     * 点击元素
     */
    async click(): Promise<this> {
        await this.locator.click({ timeout: this.options.timeout });
        return this;
    }

    /**
     * 双击元素
     */
    async dblClick(): Promise<this> {
        await this.locator.dblclick({ timeout: this.options.timeout });
        return this;
    }

    /**
     * 右键点击
     */
    async rightClick(): Promise<this> {
        await this.locator.click({
            button: "right",
            timeout: this.options.timeout,
        });
        return this;
    }

    /**
     * 填充文本（清空后填充）
     */
    async fill(text: string): Promise<this> {
        await this.locator.fill(text, { timeout: this.options.timeout });
        return this;
    }

    /**
     * 清空输入框
     */
    async clear(): Promise<this> {
        await this.locator.fill("", { timeout: this.options.timeout });
        return this;
    }

    /**
     * 逐字输入文本
     */
    async type(text: string, delay: number = 50): Promise<this> {
        await this.locator.type(text, { delay, timeout: this.options.timeout });
        return this;
    }

    /**
     * 按下按键
     */
    async press(key: string): Promise<this> {
        await this.locator.press(key, { timeout: this.options.timeout });
        return this;
    }

    /**
     * 获取文本内容
     */
    async getText(): Promise<string> {
        return await this.locator.textContent({
            timeout: this.options.timeout,
        }) || "";
    }

    /**
     * 获取属性值
     */
    async getAttribute(name: string): Promise<string | null> {
        return await this.locator.getAttribute(name, {
            timeout: this.options.timeout,
        });
    }

    /**
     * 获取输入框的值
     */
    async getValue(): Promise<string> {
        return await this.locator.inputValue({ timeout: this.options.timeout });
    }

    /**
     * 获取 Page 对象（用于 evaluate 等操作）
     */
    getPage(): any {
        return this.page;
    }

    /**
     * 检查元素是否可见
     */
    async isVisible(): Promise<boolean> {
        try {
            await this.locator.waitFor({ state: "visible", timeout: 2000 });
            return true;
        } catch {
            return false;
        }
    }

    /**
     * 检查元素是否存在
     */
    async exists(): Promise<boolean> {
        try {
            await this.locator.first().waitFor({ timeout: 1000 });
            return true;
        } catch {
            return false;
        }
    }

    /**
     * 检查元素是否禁用
     */
    async isDisabled(): Promise<boolean> {
        const disabled = await this.locator.evaluate((el: any) => el.disabled);
        return disabled === true;
    }

    /**
     * 检查元素是否启用
     */
    async isEnabled(): Promise<boolean> {
        try {
            return await this.locator.isEnabled();
        } catch (error) {
            console.warn("isEnabled 检查失败", error);
            return false;
        }
    }

    /**
     * 获取元素数量
     */
    async count(): Promise<number> {
        return await this.locator.count();
    }

    /**
     * 等待元素可见
     */
    async waitForVisible(
        timeout: number = this.options.timeout,
    ): Promise<this> {
        await this.locator.waitFor({ state: "visible", timeout });
        return this;
    }

    /**
     * 等待元素隐藏
     */
    async waitForHidden(timeout: number = this.options.timeout): Promise<this> {
        await this.locator.waitFor({ state: "hidden", timeout });
        return this;
    }

    /**
     * 滚动到元素
     */
    async scrollIntoView(): Promise<this> {
        await this.locator.scrollIntoViewIfNeeded();
        return this;
    }

    /**
     * 悬停在元素上
     */
    async hover(): Promise<this> {
        await this.locator.hover({ timeout: this.options.timeout });
        return this;
    }

    /**
     * 聚焦元素
     */
    async focus(): Promise<this> {
        await this.locator.focus({ timeout: this.options.timeout });
        return this;
    }

    /**
     * 获取元素的 CSS 属性
     */
    async getCSSProperty(propertyName: string): Promise<string> {
        return await this.locator.evaluate(
            (el: any, prop: string) =>
                window.getComputedStyle(el).getPropertyValue(prop),
            propertyName,
        );
    }

    /**
     * 截图
     */
    async screenshot(path?: string): Promise<Buffer> {
        return await this.locator.screenshot({ path });
    }

    /**
     * 等待某个条件满足（自定义断言）
     */
    async waitFor(
        condition: () => Promise<boolean>,
        timeout: number = this.options.timeout,
    ): Promise<this> {
        const startTime = Date.now();
        while (Date.now() - startTime < timeout) {
            if (await condition()) {
                return this;
            }
            await this.page.waitForTimeout(100);
        }
        throw new Error(`Timeout waiting for condition after ${timeout}ms`);
    }

    /**
     * 执行自定义断言
     */
    expect(assertion: (locator: Locator) => Promise<void>): Promise<void> {
        return assertion(this.locator);
    }
}

/**
 * 输入框元素类
 */
export class InputElement extends BaseElement {
    async fill(text: string): Promise<this> {
        await this.locator.fill(text);
        return this;
    }

    async clear(): Promise<this> {
        await this.locator.fill("");
        return this;
    }

    async selectAll(): Promise<this> {
        await this.locator.press("Control+A");
        return this;
    }
}

/**
 * 按钮元素类
 */
export class ButtonElement extends BaseElement {
    async click(): Promise<this> {
        await this.locator.click({
            timeout: this.options.timeout,
            force: true, // 避免被临时 toast/遮罩拦截
        });
        return this;
    }

    async clickMultipleTimes(
        count: number,
        delay: number = 200,
    ): Promise<this> {
        for (let i = 0; i < count; i++) {
            await this.click();
            if (i < count - 1) {
                await this.page.waitForTimeout(delay);
            }
        }
        return this;
    }

    async isLoading(): Promise<boolean> {
        const ariaLabel = await this.locator.getAttribute("aria-label");
        return ariaLabel?.includes("loading") ?? false;
    }
}

/**
 * 下拉框元素类
 */
export class SelectElement extends BaseElement {
    async selectByLabel(label: string): Promise<this> {
        await this.locator.selectOption(label);
        return this;
    }

    async selectByValue(value: string): Promise<this> {
        await this.locator.selectOption(value);
        return this;
    }

    async getSelectedValue(): Promise<string> {
        return await this.locator.inputValue();
    }

    async getSelectedText(): Promise<string> {
        return await this.locator.locator("option:checked").textContent() || "";
    }
}

/**
 * 复选框元素类
 */
export class CheckboxElement extends BaseElement {
    async check(): Promise<this> {
        await this.locator.check({ timeout: this.options.timeout });
        return this;
    }

    async uncheck(): Promise<this> {
        await this.locator.uncheck({ timeout: this.options.timeout });
        return this;
    }

    async isChecked(): Promise<boolean> {
        return await this.locator.isChecked();
    }

    async toggle(): Promise<this> {
        const checked = await this.isChecked();
        if (checked) {
            await this.uncheck();
        } else {
            await this.check();
        }
        return this;
    }
}

/**
 * 下拉菜单元素类（Combobox）
 */
export class ComboboxElement extends BaseElement {
    async open(): Promise<this> {
        // 等待元素可见并准备就绪，增加超时时间
        try {
            await this.locator.waitFor({ state: "visible", timeout: 10000 });
        } catch (e) {
            // 如果超时，尝试滚动到元素
            await this.locator.scrollIntoViewIfNeeded();
            await this.page.waitForTimeout(500);
        }

        // 点击打开 combobox
        await this.locator.click({ timeout: 10000 }).catch(async () => {
            // 如果第一次失败，尝试重新滚动和点击
            await this.locator.scrollIntoViewIfNeeded();
            await this.page.waitForTimeout(300);
            await this.locator.click({ timeout: 5000 });
        });

        await this.page.waitForTimeout(500);
        return this;
    }

    async selectOption(label: string): Promise<this> {
        await this.open();

        // 使用 getByRole('option') 来查找选项，这与 Radix UI combobox 兼容
        // 添加重试机制以处理选项查找失败的情况
        let option = this.page.getByRole("option", {
            name: new RegExp(label),
        }).first();

        try {
            await option.click({ timeout: 5000 });
        } catch (e) {
            // 如果第一次查找失败，等待一下后重试
            await this.page.waitForTimeout(1000);
            option = this.page.getByRole("option", {
                name: new RegExp(label),
            }).first();
            await option.click({ timeout: 5000 });
        }

        await this.page.waitForTimeout(300);
        return this;
    }

    async getSelectedText(): Promise<string> {
        return await this.getText();
    }
}

/**
 * SpinbuttonElement - 数字输入框元素
 */
export class SpinbuttonElement extends BaseElement {
    async setValue(value: number): Promise<this> {
        await this.locator.fill(String(value));
        await this.page.waitForTimeout(200);
        return this;
    }

    /**
     * 获取数字值
     */
    async getNumberValue(): Promise<number> {
        const value = await this.locator.inputValue();
        return parseInt(value, 10);
    }

    /**
     * 获取字符串值（继承自 BaseElement）
     */
    async getValue(): Promise<string> {
        return await this.locator.inputValue();
    }

    async increment(times: number = 1): Promise<this> {
        for (let i = 0; i < times; i++) {
            await this.locator.press("ArrowUp");
        }
        await this.page.waitForTimeout(200);
        return this;
    }

    async decrement(times: number = 1): Promise<this> {
        for (let i = 0; i < times; i++) {
            await this.locator.press("ArrowDown");
        }
        await this.page.waitForTimeout(200);
        return this;
    }
}
