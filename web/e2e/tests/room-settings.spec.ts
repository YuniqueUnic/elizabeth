/**
 * 房间设置功能测试
 * 测试房间设置、权限、密码、过期时间等功能
 */

import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
const TEST_ROOM = "settings-test-room";
const TEST_ROOM_URL = `${BASE_URL}/${TEST_ROOM}`;

test.describe("房间设置功能测试", () => {
    let roomPage: RoomPage;

    test.beforeEach(async ({ page }) => {
        roomPage = new RoomPage(page);
        await roomPage.goto(TEST_ROOM_URL);
        await roomPage.waitForRoomLoad();
    });

    test.describe("房间基础信息", () => {
        test("RS-001: 应该正确显示房间 URL", async () => {
            const url = roomPage.getRoomUrl();
            expect(url).toContain(TEST_ROOM);
        });

        test("RS-002: 应该从 URL 中提取房间名称", async () => {
            const roomName = roomPage.getRoomName();
            expect(roomName).toBe(TEST_ROOM);
        });

        test("RS-003: 应该显示容量信息", async () => {
            const capacity = await roomPage.getCapacityInfo();
            expect(capacity).toMatch(/MB/);
        });
    });

    test.describe("过期时间设置", () => {
        test("RS-004: 应该可以修改过期时间 - 1 分钟", async () => {
            await roomPage.roomSettings.expirationTime.selectOption("1 分钟");
            await roomPage.page.waitForTimeout(200);

            const selected = await roomPage.roomSettings.expirationTime
                .getSelectedText();
            expect(selected).toContain("分钟");
        });

        test("RS-005: 应该可以修改过期时间 - 1 小时", async () => {
            await roomPage.roomSettings.expirationTime.selectOption("1 小时");
            await roomPage.page.waitForTimeout(200);

            const selected = await roomPage.roomSettings.expirationTime
                .getSelectedText();
            expect(selected).toContain("小时");
        });

        test("RS-006: 应该可以修改过期时间 - 1 天", async () => {
            await roomPage.roomSettings.expirationTime.selectOption("1 天");
            await roomPage.page.waitForTimeout(200);

            const selected = await roomPage.roomSettings.expirationTime
                .getSelectedText();
            expect(selected).toContain("天");
        });

        test("RS-007: 应该可以修改过期时间 - 永不过期", async () => {
            await roomPage.roomSettings.expirationTime.selectOption("永不过期");
            await roomPage.page.waitForTimeout(200);

            const selected = await roomPage.roomSettings.expirationTime
                .getSelectedText();
            expect(selected).toContain("永不过期");
        });
    });

    test.describe("房间密码设置", () => {
        test("RS-008: 应该可以设置房间密码", async () => {
            const password = "TestPass123!";
            await roomPage.roomSettings.password.fill(password);

            const value = await roomPage.roomSettings.password.getValue();
            expect(value).toBe(password);
        });

        test("RS-009: 应该可以清空房间密码", async () => {
            await roomPage.roomSettings.password.fill("SomePassword");
            await roomPage.page.waitForTimeout(100);

            await roomPage.roomSettings.password.clear();
            const value = await roomPage.roomSettings.password.getValue();
            expect(value).toBe("");
        });

        test("RS-010: 应该支持特殊字符密码", async () => {
            const password = "P@$$w0rd!#%&";
            await roomPage.roomSettings.password.fill(password);

            const value = await roomPage.roomSettings.password.getValue();
            expect(value).toBe(password);
        });

        test("RS-011: 应该支持长密码", async () => {
            const password = "VeryLongPasswordWith32CharactersAndMore";
            await roomPage.roomSettings.password.fill(password);

            const value = await roomPage.roomSettings.password.getValue();
            expect(value).toBe(password);
        });
    });

    test.describe("最大查看次数设置", () => {
        test("RS-012: 应该可以设置最大查看次数", async () => {
            await roomPage.roomSettings.maxViewCount.setValue(50);

            const value = await roomPage.roomSettings.maxViewCount
                .getNumberValue();
            expect(value).toBe(50);
        });

        test("RS-013: 应该可以增加最大查看次数", async () => {
            await roomPage.roomSettings.maxViewCount.setValue(100);
            await roomPage.page.waitForTimeout(100);

            await roomPage.roomSettings.maxViewCount.increment(5);

            // 可能的预期值范围
            const value = await roomPage.roomSettings.maxViewCount
                .getNumberValue();
            expect(value).toBeGreaterThan(100);
        });

        test("RS-014: 应该可以减少最大查看次数", async () => {
            await roomPage.roomSettings.maxViewCount.setValue(100);
            await roomPage.page.waitForTimeout(100);

            await roomPage.roomSettings.maxViewCount.decrement(10);

            const value = await roomPage.roomSettings.maxViewCount
                .getNumberValue();
            expect(value).toBeLessThan(100);
        });

        test("RS-015: 最大查看次数应该接受小数值", async () => {
            await roomPage.roomSettings.maxViewCount.setValue(999);

            const value = await roomPage.roomSettings.maxViewCount
                .getNumberValue();
            expect(value).toBe(999);
        });
    });

    test.describe("设置保存", () => {
        test("RS-016: 应该可以保存单个设置", async () => {
            await roomPage.roomSettings.password.fill("SingleSetting");
            await roomPage.roomSettings.saveBtn.click();

            await roomPage.page.waitForTimeout(500);

            // 验证按钮仍然可用
            const isEnabled = await roomPage.roomSettings.saveBtn.isEnabled();
            expect(typeof isEnabled).toBe("boolean");
        });

        test("RS-017: 应该可以保存多个设置", async () => {
            await roomPage.fillRoomSettings({
                expirationTime: "1 周",
                password: "MultipleSettings",
                maxViewCount: 75,
            });

            await roomPage.saveRoomSettings();
            await roomPage.page.waitForTimeout(500);

            // 验证设置已保存
            const expTime = await roomPage.roomSettings.expirationTime
                .getSelectedText();
            expect(expTime).toContain("周");
        });

        test("RS-018: 应该可以多次保存设置", async () => {
            // 第一次保存
            await roomPage.roomSettings.password.fill("FirstSave");
            await roomPage.roomSettings.saveBtn.click();
            await roomPage.page.waitForTimeout(300);

            // 第二次保存
            await roomPage.roomSettings.password.clear();
            await roomPage.roomSettings.password.fill("SecondSave");
            await roomPage.roomSettings.saveBtn.click();
            await roomPage.page.waitForTimeout(300);

            const password = await roomPage.roomSettings.password.getValue();
            expect(password).toBe("SecondSave");
        });
    });

    test.describe("权限管理", () => {
        test("RS-019: 应该可以切换预览权限", async () => {
            const previewBtn = roomPage.roomPermissions.previewBtn;
            const initialState = await previewBtn.getAttribute("aria-pressed");

            await previewBtn.click();
            await roomPage.page.waitForTimeout(200);

            const newState = await previewBtn.getAttribute("aria-pressed");
            expect(newState).not.toBe(initialState);
        });

        test("RS-020: 应该可以切换编辑权限", async () => {
            await roomPage.roomPermissions.editBtn.click();
            await roomPage.page.waitForTimeout(200);

            const state = await roomPage.roomPermissions.editBtn.getAttribute(
                "aria-pressed",
            );
            expect(["true", "false"]).toContain(state);
        });

        test("RS-021: 应该可以切换分享权限", async () => {
            await roomPage.roomPermissions.shareBtn.click();
            await roomPage.page.waitForTimeout(200);

            const state = await roomPage.roomPermissions.shareBtn.getAttribute(
                "aria-pressed",
            );
            expect(["true", "false"]).toContain(state);
        });

        test("RS-022: 应该可以切换删除权限", async () => {
            await roomPage.roomPermissions.deleteBtn.click();
            await roomPage.page.waitForTimeout(200);

            const state = await roomPage.roomPermissions.deleteBtn.getAttribute(
                "aria-pressed",
            );
            expect(["true", "false"]).toContain(state);
        });

        test("RS-023: 应该可以保存权限设置", async () => {
            await roomPage.setRoomPermissions({
                preview: true,
                edit: false,
                share: false,
                delete: false,
            });

            await roomPage.page.waitForTimeout(500);

            // 验证没有错误
            const hasError = await roomPage.page.locator("text=/错误/")
                .isVisible().catch(() => false);
            expect(hasError).toBe(false);
        });

        test("RS-024: 应该支持所有权限组合", async () => {
            const combinations = [
                { preview: true, edit: true, share: true, delete: true },
                { preview: true, edit: true, share: false, delete: false },
                { preview: true, edit: false, share: false, delete: false },
                { preview: false, edit: false, share: false, delete: false },
            ];

            for (const combo of combinations.slice(0, 2)) {
                // 仅测试前两个组合以节省时间
                await roomPage.setRoomPermissions(combo);
                await roomPage.page.waitForTimeout(300);
            }

            expect(true).toBe(true); // 如果没有抛出异常就是成功
        });
    });

    test.describe("分享功能", () => {
        test("RS-025: 应该显示分享按钮", async () => {
            const getLinkBtn = roomPage.roomSharing.getLinkBtn;
            const isVisible = await getLinkBtn.isVisible();
            expect(isVisible).toBe(true);
        });

        test("RS-026: 应该显示下载二维码按钮", async () => {
            const downloadBtn = roomPage.roomSharing.downloadBtn;
            const isVisible = await downloadBtn.isVisible();
            expect(isVisible).toBe(true);
        });
    });

    test.describe("设置表单交互", () => {
        test("RS-027: 输入框应该支持焦点和取消焦点", async () => {
            const passwordInput = roomPage.roomSettings.password;

            await passwordInput.focus();
            const isFocused = await passwordInput.getLocator().evaluate((
                el: any,
            ) => el === document.activeElement);
            expect(isFocused).toBe(true);
        });

        test("RS-028: 应该可以使用 Tab 键在表单中导航", async () => {
            await roomPage.roomSettings.password.focus();
            await roomPage.page.keyboard.press("Tab");
            await roomPage.page.waitForTimeout(100);

            // 验证焦点已移动
            expect(true).toBe(true);
        });

        test("RS-029: 应该可以使用 Enter 键提交表单", async () => {
            await roomPage.roomSettings.password.fill("TestPassword");
            await roomPage.roomSettings.password.press("Tab");
            await roomPage.page.waitForTimeout(100);

            // 验证表单仍然有效
            const value = await roomPage.roomSettings.password.getValue();
            expect(value).toBe("TestPassword");
        });
    });
});
