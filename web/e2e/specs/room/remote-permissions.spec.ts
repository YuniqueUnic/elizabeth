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
  TrySaveMessages,
} from "../../screenplay/room/tasks/Room.tasks";
import { CallElizabethApi } from "../../screenplay/abilities/CallElizabethApi.ability";
import type { CapabilityGrant } from "../../screenplay/abilities/CallElizabethApi.ability";

const READER_FULL: CapabilityGrant[] = [
  { capability: "msg.read", scope: "any" },
  { capability: "msg.send", scope: "any" },
  { capability: "msg.copy", scope: "any" },
  { capability: "msg.edit", scope: "any" },
  { capability: "msg.delete", scope: "any" },
  { capability: "msg.visibility.manage", scope: "any" },
  { capability: "file.list", scope: "any" },
  { capability: "file.preview", scope: "any" },
  { capability: "file.download", scope: "any" },
];

const READER_READONLY: CapabilityGrant[] = [
  { capability: "msg.read", scope: "any" },
  { capability: "msg.copy", scope: "any" },
  { capability: "file.list", scope: "any" },
  { capability: "file.preview", scope: "any" },
  { capability: "file.download", scope: "any" },
];

test.describe("Remote room permission downgrades", () => {
  let room: ProvisionedRoom;
  let api: CallElizabethApi;
  let adminToken: string;

  test.beforeEach(async ({ actor, provisionRoom, request }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-remote-permissions"),
    });
    api = CallElizabethApi.using(request);
    adminToken = room.tokenInfo!.token;

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("role matrix changes apply live: reader regains and loses save rights without rejoining", async ({
    createActor,
  }) => {
    const remoteUser = await createActor("remote permission downgraded user");
    await remoteUser.actor.attemptsTo(OpenRoom(room.url));

    // 初始 reader 无编辑权：草稿保留在本地，保存被拒
    const draft = `Local draft after remote downgrade ${Date.now()}`;
    await RoomScreen.messageInput(remoteUser.page).fill(draft);
    await expect.poll(async () => remoteUser.actor.answer(DraftMessageText()))
      .toContain(draft);

    await remoteUser.actor.attemptsTo(SendCurrentDraft());
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBeGreaterThan(0);

    await remoteUser.actor.attemptsTo(TrySaveMessages());
    await expect(RoomScreen.toast(remoteUser.page)).toContainText(
      tCommon("permissionDenied.messageSaveEdit"),
    );
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBeGreaterThan(0);

    // 提升为可编辑 → 保存成功，草稿落库
    expect(
      await api.updateRole(room.name, "reader", READER_FULL, "Reader", adminToken),
    ).toBe(200);
    await expect.poll(
      async () => remoteUser.actor.answer(PermissionState("edit")),
      { timeout: 15_000 },
    ).toBe(true);

    await remoteUser.actor.attemptsTo(TrySaveMessages());
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBe(0);
    await expect(RoomScreen.messageItems(remoteUser.page).filter({ hasText: draft })).toBeVisible();

    // 远端再降级 → 本地权限位实时翻转，新保存再次被拒且草稿不丢
    expect(
      await api.updateRole(room.name, "reader", READER_READONLY, "Reader", adminToken),
    ).toBe(200);
    await expect.poll(
      async () => remoteUser.actor.answer(PermissionState("edit")),
      { timeout: 15_000 },
    ).toBe(false);

    await RoomScreen.messageInput(remoteUser.page).fill(`${draft} - after downgrade`);
    await remoteUser.actor.attemptsTo(SendCurrentDraft());
    await expect.poll(async () => remoteUser.actor.answer(UnsavedBadgeCount()))
      .toBeGreaterThan(0);

    await remoteUser.actor.attemptsTo(TrySaveMessages());
    await expect(RoomScreen.toast(remoteUser.page)).toContainText(
      tCommon("permissionDenied.messageSaveEdit"),
    );
    await expect(RoomScreen.messageInput(remoteUser.page)).toBeVisible();
    await expect.poll(async () => remoteUser.actor.answer(DraftMessageText()))
      .toContain("after downgrade");
  });
});
