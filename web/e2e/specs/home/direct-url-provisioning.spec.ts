import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { CallElizabethApi } from "../../screenplay/abilities/CallElizabethApi.ability";
import {
  CurrentRoomName,
  DisclosedIdentityCode,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  EnterRoomAfterDisclosure,
  OpenRoom,
  OpenUnprovisionedRoom,
  RedeemIdentityCodeInRoom,
  VisitRoomUrl,
} from "../../screenplay/room/tasks/Room.tasks";
import { uniqueRoomName } from "../../screenplay/support/test-data";

/**
 * #189 回归：URL 直达房间必须完成与首页创建一致的 admin 初始化。
 * 后端 GET /rooms/{name} 是纯查询；开通由前端显式调用创建命令完成。
 */
test.describe("Direct-URL room provisioning", () => {
  test("a fresh session can recover the admin role with the disclosed code", async ({
    createActor,
    actor,
    page,
  }) => {
    const roomName = uniqueRoomName("screenplay-direct-recovery");

    await actor.attemptsTo(OpenUnprovisionedRoom(`/${roomName}`));
    const disclosedCode = await actor.answer(DisclosedIdentityCode());
    expect(disclosedCode).not.toBe("");
    await actor.attemptsTo(EnterRoomAfterDisclosure());
    await expect(RoomScreen.membersButton(page)).toBeVisible();

    // Simulate the creator losing their browser session (fresh context):
    // the one-time code is the only durable credential to regain admin.
    const returning = await createActor("Returning");
    await returning.actor.attemptsTo(OpenRoom(`/${roomName}`));
    await expect(RoomScreen.membersButton(returning.page)).toHaveCount(0);

    await returning.actor.attemptsTo(RedeemIdentityCodeInRoom(disclosedCode));
    await expect(RoomScreen.membersButton(returning.page)).toBeVisible();
    expect(await returning.actor.answer(CurrentRoomName())).toBe(roomName);
  });

  test("reopening the provisioned room URL keeps the admin session", async ({
    actor,
    page,
  }) => {
    const roomName = uniqueRoomName("screenplay-direct-reopen");

    await actor.attemptsTo(OpenUnprovisionedRoom(`/${roomName}`));
    await actor.attemptsTo(EnterRoomAfterDisclosure());
    await expect(RoomScreen.membersButton(page)).toBeVisible();

    await page.reload();
    await expect(RoomScreen.membersButton(page)).toBeVisible();
    await expect(RoomScreen.identityCodeDisclosure(page)).toHaveCount(0);
  });

  test("a protected room URL still requires its password for visitors", async ({
    createActor,
    provisionRoom,
  }) => {
    const password = "direct-protect-1"; // pragma: allowlist secret
    const room = await provisionRoom({
      injectToken: false,
      password,
      roomName: uniqueRoomName("screenplay-direct-protected"),
    });
    const visitor = await createActor("Visitor");

    await visitor.actor.attemptsTo(VisitRoomUrl(room.url));
    await expect(RoomScreen.passwordDialogInput(visitor.page)).toBeVisible({ timeout: 15_000 });
    await expect(RoomScreen.identityCodeDisclosure(visitor.page)).toHaveCount(0);
  });

  test("provisioning conflicts converge on the existing room", async ({
    createActor,
    request,
  }) => {
    const api = CallElizabethApi.using(request);
    const roomName = uniqueRoomName("screenplay-direct-conflict");

    // A concurrent creator wins the race; the URL visit must not error —
    // it falls back to the normal existing-room entry flow.
    await api.ensureRoom(roomName);
    const latecomer = await createActor("Latecomer");
    await latecomer.actor.attemptsTo(OpenRoom(`/${roomName}`));

    await expect(RoomScreen.messageInput(latecomer.page)).toBeVisible();
    expect(await latecomer.actor.answer(CurrentRoomName())).toBe(roomName);
    await expect(RoomScreen.identityCodeDisclosure(latecomer.page)).toHaveCount(0);
  });
});
