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

  test("long unbroken text stays inside the visible message list viewport when the room settings sidebar is expanded", async ({
    actor,
    page,
  }) => {
    const longText = "a".repeat(500);
    await actor.attemptsTo(SendMessage(longText));
    await actor.attemptsTo(SaveMessages());

    await expect(RoomScreen.leftSidebar(page)).toBeVisible();
    await RoomScreen.leftSidebarCollapseButton(page).click();
    await expect(RoomScreen.leftSidebarCollapsedRail(page)).toBeVisible();
    await RoomScreen.leftSidebarExpandButton(page).click();
    await expect(RoomScreen.leftSidebar(page)).toBeVisible();

    const messageListViewport = RoomScreen.messageListViewport(page);
    const messageItem = RoomScreen.messageItems(page).last();

    const viewportBox = await messageListViewport.boundingBox();
    const itemBox = await messageItem.boundingBox();

    expect(viewportBox).not.toBeNull();
    expect(itemBox).not.toBeNull();
    expect(itemBox!.x).toBeGreaterThanOrEqual(viewportBox!.x - 1);
    expect(itemBox!.x + itemBox!.width).toBeLessThanOrEqual(
      viewportBox!.x + viewportBox!.width + 1,
    );
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

  test("ordered list markers in rendered messages keep 20+ items clear of the left edge", async ({
    actor,
    page,
  }) => {
    const orderedListMarkdown = Array.from(
      { length: 24 },
      (_, index) => `${index + 1}. Ordered item ${index + 1}`,
    ).join("\n");

    await RoomScreen.sourceModeButton(page).click();
    await RoomScreen.sourceEditor(page).fill(orderedListMarkdown);
    await RoomScreen.sendButton(page).click();
    await actor.attemptsTo(SaveMessages());

    const messageContent = RoomScreen.messageContents(page).last();
    const twentiethItem = messageContent.locator(".ProseMirror ol > li").nth(19);

    await expect(twentiethItem).toBeVisible();

    const metrics = await twentiethItem.evaluate((element) => {
      const firstTextNode = (() => {
        const queue: Node[] = [element];
        while (queue.length > 0) {
          const current = queue.shift();
          if (!current) break;
          if (
            current.nodeType === Node.TEXT_NODE &&
            current.textContent?.trim()
          ) {
            return current as Text;
          }
          queue.unshift(...Array.from(current.childNodes));
        }
        return null;
      })();

      const range = document.createRange();
      if (firstTextNode) {
        range.setStart(firstTextNode, 0);
        range.setEnd(firstTextNode, Math.min(1, firstTextNode.length));
      } else {
        range.selectNodeContents(element);
      }

      const messageContentElement = element.closest(
        '[data-testid^="message-content-"]',
      ) as HTMLElement | null;
      const proseMirror = element.closest(".ProseMirror") as HTMLElement | null;
      const textRect = range.getBoundingClientRect();
      const list = element.parentElement as HTMLOListElement | null;
      const listRect = list?.getBoundingClientRect();
      const listStyle = list ? getComputedStyle(list) : getComputedStyle(element);
      const itemStyle = getComputedStyle(element);
      const fontSize = Number.parseFloat(itemStyle.fontSize) || 14;

      const canvas = document.createElement("canvas");
      const context = canvas.getContext("2d");
      const markerText = "20.";
      let markerWidth = fontSize;
      let gapWidth = fontSize * 0.5;

      if (context) {
        context.font = itemStyle.font;
        markerWidth = context.measureText(markerText).width;
        gapWidth = Math.max(
          context.measureText("  ").width,
          fontSize * 0.5,
        );
      }

      return {
        estimatedMarkerLeft: textRect.left - markerWidth - gapWidth,
        listPaddingLeft: Number.parseFloat(listStyle.paddingLeft),
        messageContentLeft: messageContentElement?.getBoundingClientRect().left ?? 0,
        proseMirrorLeft: proseMirror?.getBoundingClientRect().left ?? 0,
        textOffsetLeftWithinList: listRect ? textRect.left - listRect.left : 0,
        requiredIndent: markerWidth + gapWidth,
      };
    });

    expect(metrics.listPaddingLeft).toBeGreaterThanOrEqual(
      metrics.requiredIndent - 1,
    );
    expect(metrics.textOffsetLeftWithinList).toBeGreaterThanOrEqual(
      metrics.requiredIndent - 1,
    );
    expect(metrics.estimatedMarkerLeft).toBeGreaterThanOrEqual(
      metrics.proseMirrorLeft - 1,
    );
    expect(metrics.estimatedMarkerLeft).toBeGreaterThanOrEqual(
      metrics.messageContentLeft - 1,
    );
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
