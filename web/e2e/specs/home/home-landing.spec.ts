import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { HomeScreen } from "../../screenplay/home/screens/Home.screen";
import {
  CreateRoomFromHome,
  NavigateBackFromJoinForm,
  VisitHomePage,
} from "../../screenplay/home/tasks/Home.tasks";
import { CurrentRoomName, DisclosedIdentityCode } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { OpenRoom, OpenUnprovisionedRoom } from "../../screenplay/room/tasks/Room.tasks";
import { EnterRoomAfterDisclosure } from "../../screenplay/room/tasks/Room.tasks";
import { API_BASE_URL } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";

test.describe("Home landing", () => {
  test("renders the home entry points", async ({ actor, page }) => {
    await actor.attemptsTo(VisitHomePage());

    await expect(HomeScreen.title(page)).toBeVisible();
    await expect(HomeScreen.subtitle(page)).toBeVisible();
    await expect(HomeScreen.createRoomCard(page)).toBeVisible();
    await expect(HomeScreen.joinRoomCard(page)).toBeVisible();
  });

  test("creates a room without a password from home", async ({ actor, page }) => {
    const roomName = uniqueRoomName("screenplay-home-basic");

    await actor.attemptsTo(CreateRoomFromHome(roomName));

    await expect(RoomScreen.messageInput(page)).toBeVisible();
    expect(await actor.answer(CurrentRoomName())).toBe(roomName);
  });

  // Product contract: opening a valid missing room URL provisions the room
  // through the create command. The creator receives an admin session and the
  // one-time admin identity code instead of a silent default-role session.
  test("provisions a missing room with admin credentials when its URL is opened", async ({
    actor,
    page,
  }) => {
    const roomName = uniqueRoomName("screenplay-direct-room");

    await actor.attemptsTo(OpenUnprovisionedRoom(`/${roomName}`));

    await expect(RoomScreen.identityCodeDisclosure(page)).toBeVisible();
    const disclosedCode = await actor.answer(DisclosedIdentityCode());
    expect(disclosedCode).not.toBe("");

    await actor.attemptsTo(EnterRoomAfterDisclosure());

    await expect(RoomScreen.messageInput(page)).toBeVisible();
    await expect(RoomScreen.membersButton(page)).toBeVisible();
    expect(await actor.answer(CurrentRoomName())).toBe(roomName);

    const response = await page.request.get(
      `${API_BASE_URL}/rooms/${roomName}`,
    );
    expect(response.status()).toBe(200);
    const room = await response.json();
    expect(room).toMatchObject({
      name: roomName,
      slug: roomName,
      password_protected: false,
      max_size: 50 * 1024 * 1024,
      max_times_entered: 100,
      default_role_key: "reader",
    });
    const lifetimeSeconds = Math.round(
      (Date.parse(room.expire_at) - Date.parse(room.created_at)) / 1000,
    );
    expect(lifetimeSeconds).toBeGreaterThanOrEqual(7199);
    expect(lifetimeSeconds).toBeLessThanOrEqual(7200);
  });

  test("returns from the join form back to the landing page", async ({ actor, page }) => {
    await actor.attemptsTo(NavigateBackFromJoinForm());

    await expect(HomeScreen.title(page)).toBeVisible();
    await expect(HomeScreen.joinRoomNameInput(page)).toHaveCount(0);
  });
});
