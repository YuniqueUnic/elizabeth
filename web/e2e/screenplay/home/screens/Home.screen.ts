import type { Locator, Page } from "@playwright/test";

export const HomeScreen = {
  title: (page: Page): Locator =>
    page.getByRole("heading", { name: /Elizabeth/i }),

  subtitle: (page: Page): Locator =>
    page.getByText("安全、临时、可控的文件分享与协作平台"),

  createRoomCard: (page: Page): Locator =>
    page.getByText("创建房间").first(),

  joinRoomCard: (page: Page): Locator =>
    page.getByText("加入房间").first(),

  roomNameInput: (page: Page): Locator =>
    page.locator("#room-name"),

  createPasswordInput: (page: Page): Locator =>
    page.locator("#password").first(),

  confirmPasswordInput: (page: Page): Locator =>
    page.locator("#confirm-password"),

  createRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: "创建房间" }),

  joinRoomNameInput: (page: Page): Locator =>
    page.locator("#join-room-name"),

  joinRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: "加入房间" }),

  backButton: (page: Page): Locator =>
    page.getByRole("button", { name: "返回" }),

  alert: (page: Page): Locator =>
    page.locator("div[role='alert'][data-slot='alert']").first(),
};
