import type { Locator, Page } from "@playwright/test";

import { tHome } from "../../support/i18n";

export const HomeScreen = {
  title: (page: Page): Locator =>
    page.getByRole("heading", { name: /Elizabeth/i }),

  subtitle: (page: Page): Locator =>
    page.getByText(tHome("platformDescription")),

  createRoomCard: (page: Page): Locator =>
    page.getByText(tHome("createRoom")).first(),

  joinRoomCard: (page: Page): Locator =>
    page.getByText(tHome("joinRoom")).first(),

  roomNameInput: (page: Page): Locator =>
    page.locator("#room-name"),

  createPasswordInput: (page: Page): Locator =>
    page.locator("#password").first(),

  confirmPasswordInput: (page: Page): Locator =>
    page.locator("#confirm-password"),

  createRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: tHome("createRoom") }),

  joinRoomNameInput: (page: Page): Locator =>
    page.locator("#join-room-name"),

  joinRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: tHome("joinRoom") }),

  backButton: (page: Page): Locator =>
    page.getByRole("button", { name: tHome("back") }),

  alert: (page: Page): Locator =>
    page.locator("div[role='alert'][data-slot='alert']").first(),
};
