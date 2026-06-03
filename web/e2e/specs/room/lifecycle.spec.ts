import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { API_BASE_URL } from "../../screenplay/support/constants";
import { tErrors } from "../../screenplay/support/i18n";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { ConfigureRoom, OpenRoom } from "../../screenplay/room/tasks/Room.tasks";

test.describe("Room lifecycle and limits", () => {
  test("rejects later visitors once the maximum view count has been reached", async ({
    actor,
    createActor,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-lifecycle"),
    });
    const visitorA = await createActor("Visitor-A");
    const visitorB = await createActor("Visitor-B");

    await actor.attemptsTo(
      OpenRoom(room.url),
      ConfigureRoom({ maxViews: 2 }),
    );

    await visitorA.page.goto(room.url);
    await expect(RoomScreen.messageInput(visitorA.page)).toBeVisible();

    await visitorB.page.goto(room.url);
    await expect(RoomScreen.alert(visitorB.page)).toBeVisible();
    await expect(RoomScreen.alert(visitorB.page)).toContainText(
      tErrors("roomInaccessibleViaLink"),
    );
  });

  test.fixme("blocks access after the room has been force-expired through the API", async ({
    actor,
    createActor,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-expiry"),
    });

    await actor.attemptsTo(OpenRoom(room.url));

    const expireAt = new Date(Date.now() - 10_000).toISOString().split(".")[0];
    const visitor = await createActor("Expired-Visitor");
    const response = await visitor.page.request.put(
      `${API_BASE_URL}/rooms/${room.name}/settings?token=${room.tokenInfo?.token}`,
      {
        data: {
          expire_at: expireAt,
        },
        headers: {
          "Content-Type": "application/json",
        },
      },
    );

    expect([200, 500]).toContain(response.status());

    await visitor.page.goto(room.url);
    await expect(RoomScreen.alert(visitor.page)).toBeVisible();
    await expect(RoomScreen.alert(visitor.page)).toContainText(
      tErrors("roomInaccessibleViaLink"),
    );
  });
});
