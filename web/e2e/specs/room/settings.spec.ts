import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  CurrentRoomName,
  PermissionState,
  RoomCapacitySummary,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  ConfigureRoom,
  OpenRoom,
  SetRoomPermissions,
  UnlockProtectedRoom,
} from "../../screenplay/room/tasks/Room.tasks";
import { tRoom } from "../../screenplay/support/i18n";

test.describe("Room settings", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-settings"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("shows the room identity and capacity summary", async ({ actor }) => {
    expect(await actor.answer(CurrentRoomName())).toBe(room.name);
    expect(await actor.answer(RoomCapacitySummary())).toContain("MB");
  });

  test("updates expiry, password, and max views in one save", async ({ actor, page }) => {
    const password = "MultipleSettings123!";

    await actor.attemptsTo(
      ConfigureRoom({
        expiry: tRoom("config.expiry.options.oneHour"),
        maxViews: 75,
        password,
      }),
    );

    await page.reload();
    const postReloadState = await Promise.race([
      RoomScreen.messageInput(page)
        .waitFor({ state: "visible", timeout: 30_000 })
        .then(() => "room" as const),
      RoomScreen.passwordDialogInput(page)
        .waitFor({ state: "visible", timeout: 30_000 })
        .then(() => "password" as const),
    ]);

    if (postReloadState === "password") {
      await actor.attemptsTo(UnlockProtectedRoom(password));
    }
    await expect(RoomScreen.messageInput(page)).toBeVisible();
    await expect(RoomScreen.roomPasswordInput(page)).toHaveValue(password);
    await expect(RoomScreen.maxViewsInput(page)).toHaveValue("75");
  });

  test("updates permission toggles and keeps their saved states", async ({ actor, page }) => {
    await actor.attemptsTo(
      SetRoomPermissions({
        delete: false,
      }),
    );

    await page.reload();
    await expect(RoomScreen.messageInput(page)).toBeVisible();

    expect(await actor.answer(PermissionState("read"))).toBe(true);
    expect(await actor.answer(PermissionState("edit"))).toBe(true);
    expect(await actor.answer(PermissionState("share"))).toBe(true);
    expect(await actor.answer(PermissionState("delete"))).toBe(false);
  });

  test("exposes the sharing controls", async ({ page }) => {
    await expect(RoomScreen.shareLinkButton(page)).toBeVisible();
    await expect(RoomScreen.shareDownloadQrButton(page)).toBeVisible();
  });
});
