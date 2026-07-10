import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { HomeScreen } from "../../screenplay/home/screens/Home.screen";
import {
  CreateRoomFromHome,
  NavigateBackFromJoinForm,
  VisitHomePage,
} from "../../screenplay/home/tasks/Home.tasks";
import { CurrentRoomName } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { OpenRoom } from "../../screenplay/room/tasks/Room.tasks";
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

  // Product contract: keep this room unprovisioned. Opening a valid missing
  // room URL must create it with the deployment defaults. Pre-creating the
  // room here would silently remove coverage for the zero-step creation UX.
  test("creates a missing room when its URL is opened directly", async ({
    actor,
    page,
  }) => {
    const roomName = uniqueRoomName("screenplay-direct-room");

    await actor.attemptsTo(
      OpenRoom(`/${roomName}`),
    );

    await expect(RoomScreen.messageInput(page)).toBeVisible();
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
      permission: 15,
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
