import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { tCommon } from "../../screenplay/support/i18n";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  DraftMessageText,
  PermissionState,
  UnsavedBadgeCount,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  OpenRoom,
  SendCurrentDraft,
  SetRoomPermissions,
  TrySaveMessages,
} from "../../screenplay/room/tasks/Room.tasks";

test.describe("Remote room permission downgrades", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-remote-permissions"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("keeps the current user's local draft while blocking cloud save after edit/delete are revoked", async ({
    actor,
    createActor,
  }) => {
    const remoteUser = await createActor("remote permission downgraded user");
    await remoteUser.actor.attemptsTo(OpenRoom(room.url));

    const draft = `Local draft after remote downgrade ${Date.now()}`;
    await RoomScreen.messageInput(remoteUser.page).fill(draft);
    await expect.poll(async () => remoteUser.actor.answer(DraftMessageText()))
      .toContain(draft);

    await actor.attemptsTo(SetRoomPermissions({ delete: false, edit: false }));

    await expect.poll(
      async () => remoteUser.actor.answer(PermissionState("edit")),
      { timeout: 15_000 },
    ).toBe(false);
    await expect.poll(
      async () => remoteUser.actor.answer(PermissionState("delete")),
      { timeout: 15_000 },
    ).toBe(false);

    await expect(RoomScreen.messageInput(remoteUser.page)).toBeVisible();
    await expect.poll(async () => remoteUser.actor.answer(DraftMessageText()))
      .toContain(draft);
    await expect(RoomScreen.fileUploadButton(remoteUser.page)).toBeDisabled();
    await expect(RoomScreen.fileAddLinkButton(remoteUser.page)).toBeDisabled();
    await expect(RoomScreen.fileUploadZone(remoteUser.page)).toHaveCount(0);

    await remoteUser.actor.attemptsTo(SendCurrentDraft());
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBeGreaterThan(0);

    await remoteUser.actor.attemptsTo(TrySaveMessages());
    await expect(RoomScreen.toast(remoteUser.page)).toContainText(
      tCommon("permissionDenied.messageSaveEdit"),
    );
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBeGreaterThan(0);
  });
});
