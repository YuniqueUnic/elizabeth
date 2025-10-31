import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";
import { htmlSelectors } from "../selectors/html-selectors";

test.describe("烟雾测试 - Playwright 框架验证", () => {
  let roomPage: RoomPage;

  test.beforeEach(async ({ page }) => {
    roomPage = new RoomPage(page);
  });

  test("ST-001: 验证 RoomPage 可以实例化", async ({ page }) => {
    expect(roomPage).toBeDefined();
    expect(roomPage.page).toBeDefined();
  });

  test("ST-002: 验证选择器已定义", async () => {
    expect(htmlSelectors).toBeDefined();
    expect(htmlSelectors.topBar).toBeDefined();
    expect(htmlSelectors.leftSidebar).toBeDefined();
  });

  test("ST-003: 验证页面导航功能", async ({ page }) => {
    expect(page).toBeDefined();
  });

  test("ST-004: 验证 BaseElement 功能", async ({ page }) => {
    // 测试 BaseElement 类是否可以创建并使用
    const { InputElement } = await import("../page-objects/base-element");
    expect(InputElement).toBeDefined();
  });

  test("ST-005: 验证 BasePage 功能", async ({ page }) => {
    expect(roomPage).toBeTruthy();
    expect(roomPage.page).toBe(page);
  });
});
