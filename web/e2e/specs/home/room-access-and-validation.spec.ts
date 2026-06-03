import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { HomeScreen } from "../../screenplay/home/screens/Home.screen";
import {
  CreateRoomFromHome,
  OpenCreateRoomForm,
  OpenJoinRoomForm,
  StartJoiningRoom,
  VisitHomePage,
} from "../../screenplay/home/tasks/Home.tasks";
import { AlertText, CurrentRoomName } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { UnlockProtectedRoom } from "../../screenplay/room/tasks/Room.tasks";
import { uniqueRoomName } from "../../screenplay/support/test-data";

test.describe("Home room access and validation", () => {
  test("creates a password-protected room from home", async ({ actor, page }) => {
    const roomName = uniqueRoomName("screenplay-home-protected");
    const password = "test123";

    await actor.attemptsTo(
      CreateRoomFromHome(roomName, {
        password,
      }),
    );

    await expect(RoomScreen.messageInput(page)).toBeVisible();
    expect(await actor.answer(CurrentRoomName())).toBe(roomName);
  });

  test("requires the correct password before a visitor can enter a protected room", async ({
    createActor,
    provisionRoom,
  }) => {
    const password = "test123";
    const room = await provisionRoom({
      injectToken: false,
      password,
      roomName: uniqueRoomName("screenplay-join-protected"),
    });
    const visitor = await createActor("Visitor");

    await visitor.actor.attemptsTo(StartJoiningRoom(room.name));

    await expect(RoomScreen.passwordDialogInput(visitor.page)).toBeVisible();
    await RoomScreen.passwordDialogInput(visitor.page).fill("wrong-password");
    await RoomScreen.passwordDialogEnterRoomButton(visitor.page).click();
    await expect(RoomScreen.passwordDialogError(visitor.page)).toBeVisible();

    await visitor.actor.attemptsTo(UnlockProtectedRoom(password));

    await expect(RoomScreen.messageInput(visitor.page)).toBeVisible();
    expect(await visitor.actor.answer(CurrentRoomName())).toBe(room.name);
  });

  test("blocks invalid room names in the create flow", async ({ actor, page }) => {
    await actor.attemptsTo(
      VisitHomePage(),
      OpenCreateRoomForm(),
    );

    await HomeScreen.roomNameInput(page).fill("/asdf/asfd/asef");
    await HomeScreen.createRoomButton(page).click();

    await expect(HomeScreen.alert(page)).toBeVisible();
    await expect(HomeScreen.alert(page)).toContainText(
      "房间名称只能包含字母、数字、下划线和连字符",
    );
    await expect(page).toHaveURL(/\/$/);
  });

  test("blocks invalid room names in the join flow", async ({ actor, page }) => {
    await actor.attemptsTo(
      VisitHomePage(),
      OpenJoinRoomForm(),
    );

    await HomeScreen.joinRoomNameInput(page).fill("asdf/asfd/asef");
    await HomeScreen.joinRoomButton(page).click();

    await expect(HomeScreen.alert(page)).toBeVisible();
    await expect(HomeScreen.alert(page)).toContainText(
      "房间名称只能包含字母、数字、下划线和连字符",
    );
    await expect(page).toHaveURL(/\/$/);
  });

  test("shows an alert for an illegal direct room slug", async ({ actor, page }) => {
    await page.goto("/-invalid-name");

    await expect(RoomScreen.alert(page)).toBeVisible();
    expect(await actor.answer(AlertText())).toContain("房间名称不合法");
  });
});
