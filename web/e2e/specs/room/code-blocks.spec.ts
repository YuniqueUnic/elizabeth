import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  OpenRoom,
  SendMessage,
} from "../../screenplay/room/tasks/Room.tasks";

test.describe("Room message code blocks", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-code-blocks"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("renders highlighted fenced code blocks in message bubbles", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      SendMessage("```javascript\nconst answer = 42;\n```"),
    );

    const latestMessage = RoomScreen.messageContents(page).last();
    await expect(latestMessage.locator("pre code.language-javascript")).toBeVisible();
    await expect(latestMessage.locator(".hljs-keyword").first()).toBeVisible();
  });

  test("inserts a code block from the editor toolbar", async ({ page }) => {
    await RoomScreen.codeBlockToolbarButton(page).click();
    const editor = RoomScreen.messageInput(page);
    await expect(editor).toBeFocused();
    await expect(editor.locator("pre")).toBeVisible();
    await editor.pressSequentially("const toolbarAnswer = 42;");
    await RoomScreen.sendButton(page).click();

    const latestMessage = RoomScreen.messageContents(page).last();
    await expect(latestMessage.locator("pre")).toBeVisible();
    await expect(latestMessage).toContainText("toolbarAnswer");
  });
});
