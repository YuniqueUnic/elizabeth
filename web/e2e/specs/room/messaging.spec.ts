import type { Route } from "@playwright/test";

import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  DraftMessageText,
  EditedBadgeCount,
  LastMessageText,
  MessageCount,
  UnsavedBadgeCount,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { tCommon, tRoom } from "../../screenplay/support/i18n";
import {
  OpenRoom,
  SaveMessages,
  SendMessage,
  SwitchToMobileViewport,
  UpdateLatestMessage,
} from "../../screenplay/room/tasks/Room.tasks";

test.describe("Room messaging", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-messaging"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("sends plain, emoji, and multiline messages", async ({ actor, page }) => {
    const initialCount = await actor.answer(MessageCount());

    await actor.attemptsTo(SendMessage("Hello World"));
    await actor.attemptsTo(SendMessage("Hello 👋 World 🌍"));
    await RoomScreen.messageInput(page).fill("Line 1\nLine 2\nLine 3");
    await RoomScreen.sendButton(page).click();

    await expect.poll(async () => actor.answer(MessageCount())).toBe(initialCount + 3);
    expect(await actor.answer(LastMessageText())).toContain("Line 1");
  });

  test("shows unsaved badges until the message list is saved", async ({ actor }) => {
    await actor.attemptsTo(SendMessage("Unsaved draft message"));

    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBeGreaterThan(0);

    await actor.attemptsTo(SaveMessages());

    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);
  });

  test("edits the latest message and marks it as edited", async ({ actor }) => {
    await actor.attemptsTo(
      SendMessage("Original message"),
      SaveMessages(),
      UpdateLatestMessage("Edited message"),
    );

    await expect.poll(async () => actor.answer(LastMessageText())).toContain("Edited message");
    await expect.poll(async () => actor.answer(EditedBadgeCount())).toBeGreaterThan(0);
  });

  test("keeps an unsent draft after switching to the mobile viewport", async ({ actor, page }) => {
    const draft = `Draft ${Date.now()}`;

    await RoomScreen.messageInput(page).fill(draft);
    await actor.attemptsTo(SwitchToMobileViewport());
    await page.getByRole("tab", { name: tCommon("mobileTabChat") }).click().catch(() => {});

    expect((await actor.answer(DraftMessageText())) ?? "").toContain("Draft");
  });

  test.fixme("shows an error toast when message saving fails", async ({ actor, page }) => {
    await page.route(`**/api/v1/rooms/${room.name}/messages`, (route: Route) => {
      if (route.request().method() === "POST") {
        return route.abort("failed");
      }

      return route.continue();
    });

    await actor.attemptsTo(
      SendMessage("This message will fail"),
      SaveMessages(),
    );

    await expect(
      page.locator("[data-state='open']").filter({
        hasText: tRoom("chat.sendFailed"),
      }),
    ).toBeVisible();
  });
});
