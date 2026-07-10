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

  test("opens an existing public room directly from its URL", async ({
    actor,
    page,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-direct-room"),
      injectToken: false,
    });

    await actor.attemptsTo(
      OpenRoom(room.url),
    );

    await expect(RoomScreen.messageInput(page)).toBeVisible();
    expect(await actor.answer(CurrentRoomName())).toBe(room.name);
  });

  test("returns from the join form back to the landing page", async ({ actor, page }) => {
    await actor.attemptsTo(NavigateBackFromJoinForm());

    await expect(HomeScreen.title(page)).toBeVisible();
    await expect(HomeScreen.joinRoomNameInput(page)).toHaveCount(0);
  });
});
