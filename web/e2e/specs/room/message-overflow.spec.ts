import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  OpenRoom,
  SaveMessages,
  SendMessage,
} from "../../screenplay/room/tasks/Room.tasks";

test.describe("Message list layout and overflow", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-overflow"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("long unbroken text does not overflow the message list width", async ({
    actor,
    page,
  }) => {
    const longText = "a".repeat(500);
    await actor.attemptsTo(SendMessage(longText));
    await actor.attemptsTo(SaveMessages());

    const messageListScroll = RoomScreen.messageListScroll(page);
    const messageItem = RoomScreen.messageItems(page).last();

    const listWidth = await messageListScroll.evaluate(
      (el) => el.getBoundingClientRect().width,
    );
    const itemWidth = await messageItem.evaluate(
      (el) => el.getBoundingClientRect().width,
    );

    // Message item should not exceed the list container width
    // Allow small tolerance for borders/padding
    expect(itemWidth).toBeLessThanOrEqual(listWidth + 4);
  });

  test("message content wraps properly with break-words", async ({
    actor,
    page,
  }) => {
    const longUrl = `https://example.com/${"path/".repeat(50)}`;
    await actor.attemptsTo(SendMessage(longUrl));
    await actor.attemptsTo(SaveMessages());

    const messageItem = RoomScreen.messageItems(page).last();
    const messageContent = messageItem.locator(
      '[data-testid^="message-content-"]',
    );

    // Content should not cause horizontal overflow in the message item
    const contentWidth = await messageContent.evaluate(
      (el) => el.scrollWidth,
    );
    const containerWidth = await messageItem.evaluate(
      (el) => el.clientWidth,
    );

    // Content scrollWidth should not exceed container width significantly
    expect(contentWidth).toBeLessThanOrEqual(containerWidth + 4);
  });

  test("message meta row does not overflow the message card", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(SendMessage("Test message for meta check"));
    await actor.attemptsTo(SaveMessages());

    const messageItem = RoomScreen.messageItems(page).last();
    const meta = messageItem.locator('[data-testid^="message-meta-"]');

    const itemBox = await messageItem.boundingBox();
    const metaBox = await meta.boundingBox();

    if (itemBox && metaBox) {
      expect(metaBox.x + metaBox.width).toBeLessThanOrEqual(
        itemBox.x + itemBox.width + 1,
      );
    }
  });

  test("code blocks in messages are scrollable horizontally", async ({
    actor,
    page,
  }) => {
    const codeMessage =
      '```javascript\nconst veryLongVariableName = "this is a very long string that should trigger horizontal scrolling in the code block container";\n```';
    await actor.attemptsTo(SendMessage(codeMessage));
    await actor.attemptsTo(SaveMessages());

    const messageItem = RoomScreen.messageItems(page).last();
    const pre = messageItem.locator("pre").first();

    if ((await pre.count()) > 0) {
      const preOverflow = await pre.evaluate(
        (el) => getComputedStyle(el).overflowX,
      );
      expect(preOverflow).toBe("auto");
    }
  });
});

test.describe("Expanded editor layout", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-editor"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("expanded editor send button has right padding", async ({
    actor,
    page,
  }) => {
    await RoomScreen.expandEditorButton(page).click();

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 5_000 });

    const sendButton = dialog.getByRole("button", {
      name: /send|发送/i,
    });

    if ((await sendButton.count()) > 0) {
      const buttonBox = await sendButton.boundingBox();
      const dialogBox = await dialog.boundingBox();

      if (buttonBox && dialogBox) {
        // The send button should have some right padding from the dialog edge
        const rightGap =
          dialogBox.x + dialogBox.width - (buttonBox.x + buttonBox.width);
        expect(rightGap).toBeGreaterThan(4);
      }
    }
  });
});
