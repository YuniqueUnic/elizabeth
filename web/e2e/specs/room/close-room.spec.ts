import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomExists } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { OpenRoom } from "../../screenplay/room/tasks/Room.tasks";

test.describe("Room closure", () => {
  test("shows the destructive confirmation immediately for rooms without a password", async ({
    actor,
    page,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-close-open"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await RoomScreen.closeRoomButton(page).click();

    await expect(RoomScreen.dialog(page)).toBeVisible();
    await expect(RoomScreen.closeRoomPasswordInput(page)).toHaveCount(0);
    await expect(RoomScreen.closeRoomConfirmButton(page)).toBeVisible();
  });

  test("cancels closing an unprotected room without deleting it", async ({
    actor,
    page,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-close-cancel"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await RoomScreen.closeRoomButton(page).click();
    await RoomScreen.closeRoomCancelButton(page).click();

    await expect(RoomScreen.dialog(page)).not.toBeVisible();
    expect(await actor.answer(RoomExists(room.name))).toBe(true);
  });

  test("rejects a wrong password before showing the destructive close step", async ({
    actor,
    page,
    provisionRoom,
  }) => {
    const password = "correct-password-123"; // pragma: allowlist secret
    const room = await provisionRoom({
      actor,
      password,
      roomName: uniqueRoomName("screenplay-close-protected"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await RoomScreen.closeRoomButton(page).click();
    await expect(RoomScreen.closeRoomPasswordInput(page)).toBeVisible();

    await RoomScreen.closeRoomPasswordInput(page).fill("wrong-password-xyz");
    await RoomScreen.closeRoomNextButton(page).click();

    await expect(RoomScreen.closeRoomPasswordError(page)).toBeVisible();
    await expect(RoomScreen.closeRoomConfirmButton(page)).toHaveCount(0);
    expect(await actor.answer(RoomExists(room.name))).toBe(true);
  });

  test("physically closes a protected room after the correct password is verified", async ({
    actor,
    page,
    provisionRoom,
  }) => {
    const password = "correct-password-123"; // pragma: allowlist secret
    const room = await provisionRoom({
      actor,
      password,
      roomName: uniqueRoomName("screenplay-close-confirm"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await RoomScreen.closeRoomButton(page).click();
    await RoomScreen.closeRoomPasswordInput(page).fill(password);
    await RoomScreen.closeRoomNextButton(page).click();
    await expect(RoomScreen.closeRoomConfirmButton(page)).toBeVisible();

    await RoomScreen.closeRoomConfirmButton(page).click();

    await expect(page).toHaveURL(/\/$/);
    await expect.poll(async () => actor.answer(RoomExists(room.name))).toBe(false);
  });
});
