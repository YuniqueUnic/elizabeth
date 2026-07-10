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

  test("keeps line numbers aligned across blank plain-text lines", async ({
    page,
  }) => {
    await RoomScreen.sourceModeButton(page).click();
    const sourceEditor = RoomScreen.sourceEditor(page);
    await expect(sourceEditor).toBeVisible();
    await sourceEditor.fill("```text\nfirst\n\nlast\n```");
    await RoomScreen.sendButton(page).click();

    const latestMessage = RoomScreen.messageContents(page).last();
    const shikiBlock = latestMessage.locator("[data-testid='shiki-code-block']");
    const lines = shikiBlock.locator("code.shiki-code > span.line");

    await expect(shikiBlock).toHaveAttribute("data-language", "text");
    await expect(lines).toHaveCount(3);

    const metrics = await lines.evaluateAll((elements) =>
      elements.map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          line: element.getAttribute("data-line"),
          top: rect.top,
          bottom: rect.bottom,
          height: rect.height,
          lineHeight: Number.parseFloat(getComputedStyle(element).lineHeight),
        };
      })
    );

    expect(metrics.map(({ line }) => line)).toEqual(["1", "2", "3"]);

    for (const metric of metrics) {
      expect(metric.height).toBeGreaterThan(0);
      expect(Math.abs(metric.height - metric.lineHeight)).toBeLessThan(1);
    }

    const spacings: number[] = [];
    for (let index = 1; index < metrics.length; index += 1) {
      const previous = metrics[index - 1];
      const current = metrics[index];

      expect(current.top).toBeGreaterThanOrEqual(previous.bottom - 0.5);
      spacings.push(current.top - previous.top);
    }

    const expectedSpacing = spacings[0];
    expect(expectedSpacing).toBeGreaterThan(0);
    for (const spacing of spacings) {
      expect(Math.abs(spacing - expectedSpacing)).toBeLessThan(1);
    }
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
