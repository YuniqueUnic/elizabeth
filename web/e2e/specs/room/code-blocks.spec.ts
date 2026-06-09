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
    await expect(latestMessage.locator("[data-testid='shiki-code-block']")).toBeVisible();
    await expect(latestMessage.locator("pre.shiki code.shiki-code")).toBeVisible();
    await expect(latestMessage.locator("code.shiki-code span[style*='color']").first()).toBeVisible();
  });

  test("inserts a code block from the editor toolbar", async ({ page }) => {
    await RoomScreen.codeBlockLanguageSelect(page).click();
    await page.getByRole("option", { name: "Rust" }).click();

    await RoomScreen.codeBlockToolbarButton(page).click();
    const editor = RoomScreen.messageInput(page);
    await expect(editor).toBeFocused();
    await expect(editor.locator("pre")).toBeVisible();
    await editor.pressSequentially('fn main() { println!("toolbar"); }');
    await RoomScreen.sendButton(page).click();

    const latestMessage = RoomScreen.messageContents(page).last();
    const shikiBlock = latestMessage.locator("[data-testid='shiki-code-block']");
    await expect(shikiBlock).toHaveAttribute("data-language", "rust");
    await expect(latestMessage.locator("pre.shiki code.shiki-code")).toBeVisible();
    await expect(latestMessage.locator("code.shiki-code span[style*='color']").first()).toBeVisible();
    await expect(latestMessage).toContainText("toolbar");
  });
});
