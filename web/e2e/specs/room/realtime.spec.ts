import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { LastMessageText } from "../../screenplay/room/questions/Room.questions";
import { OpenRoom, SaveMessages, SendMessage } from "../../screenplay/room/tasks/Room.tasks";
import { uniqueRoomName } from "../../screenplay/support/test-data";

test.describe("Room realtime sync", () => {
  test("shows a saved message from one actor in another actor's browser", async ({
    createActor,
    provisionRoom,
  }) => {
    const roomName = uniqueRoomName("screenplay-realtime");
    const actorA = await createActor("Realtime-A");
    const actorB = await createActor("Realtime-B");
    const room = await provisionRoom({
      actor: actorA.actor,
      roomName,
    });

    await provisionRoom({
      actor: actorB.actor,
      roomName,
    });

    await actorA.actor.attemptsTo(OpenRoom(room.url));
    await actorB.actor.attemptsTo(OpenRoom(room.url));

    const message = `hello-realtime-${Date.now()}`;
    await actorA.actor.attemptsTo(
      SendMessage(message),
      SaveMessages(),
    );

    await expect.poll(async () => actorB.actor.answer(LastMessageText())).toContain(message);
  });
});
