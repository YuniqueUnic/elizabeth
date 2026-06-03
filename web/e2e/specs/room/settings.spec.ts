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
} from "../../screenplay/room/tasks/Room.tasks";

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
    expect(await actor.answer(RoomCapacitySummary())).toMatch(/MB|房间占用|容量使用/);
  });

  test("updates expiry, password, and max views in one save", async ({ actor, page }) => {
    const password = "MultipleSettings123!";

    await actor.attemptsTo(
      ConfigureRoom({
        expiry: "1 小时",
        maxViews: 75,
        password,
      }),
    );

    await page.reload();
    await expect(RoomScreen.messageInput(page)).toBeVisible();
    await expect(RoomScreen.roomPasswordInput(page)).toHaveValue(password);
    await expect(RoomScreen.maxViewsInput(page)).toHaveValue("75");
  });

  test("updates permission toggles and keeps their saved states", async ({ actor, page }) => {
    await actor.attemptsTo(
      SetRoomPermissions({
        "分享": false,
        "删除": false,
        "编辑": false,
        "预览": true,
      }),
    );

    await page.reload();
    await expect(RoomScreen.messageInput(page)).toBeVisible();

    expect(await actor.answer(PermissionState("预览"))).toBe(true);
    expect(await actor.answer(PermissionState("编辑"))).toBe(false);
    expect(await actor.answer(PermissionState("分享"))).toBe(false);
    expect(await actor.answer(PermissionState("删除"))).toBe(false);
  });

  test("exposes the sharing controls", async ({ page }) => {
    await expect(RoomScreen.shareLinkButton(page)).toBeVisible();
    await expect(RoomScreen.shareDownloadQrButton(page)).toBeVisible();
  });
});
