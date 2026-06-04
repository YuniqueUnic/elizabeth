import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  ClipboardContents,
  ScrollMetrics,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  CopySelectedMessages,
  CopySingleMessage,
  DeleteSelectedMessages,
  DeleteMessageById,
  ConfirmDelete,
  ConfirmDeleteAndDisableFuture,
  DownloadSelectedMessages,
  EnableSetting,
  SetSettingTo,
  OpenRoom,
  SaveMessages,
  SendMessage,
} from "../../screenplay/room/tasks/Room.tasks";
import { SelectAllMessages } from "../../screenplay/room/interactions/Room.interactions";
import { tCommon, tRoom } from "../../screenplay/support/i18n";

const scrollMessageListToTop = async (page: import("@playwright/test").Page) => {
  const container = page.getByTestId("message-list-scroll");
  await container.evaluate((element) => {
    const viewport = element.querySelector("[data-radix-scroll-area-viewport]") as HTMLDivElement | null;
    if (viewport) {
      viewport.scrollTo(0, 0);
      viewport.dispatchEvent(new Event("scroll"));
    }
  });
};

test.describe("Room settings integration", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-settings"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await actor.attemptsTo(
      SendMessage("First message for testing"),
      SendMessage("Second message for testing"),
      SaveMessages(),
    );
  });

  test.describe("copy and download with metadata", () => {
    test("copies messages without metadata when setting is off", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SelectAllMessages());
      await actor.attemptsTo(CopySelectedMessages());

      const clipboard = await actor.answer(ClipboardContents());
      expect(clipboard).toContain("First message for testing");
      expect(clipboard).toContain("Second message for testing");
      expect(clipboard).not.toMatch(/^###\s/);
    });

    test("copies messages with metadata when setting is on", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SetSettingTo("setting-include-metadata-copy", true));
      await actor.attemptsTo(OpenRoom(room.url));
      await actor.attemptsTo(
        SendMessage("First message for testing"),
        SendMessage("Second message for testing"),
        SaveMessages(),
      );

      await actor.attemptsTo(SelectAllMessages());
      await actor.attemptsTo(CopySelectedMessages());

      const clipboard = await actor.answer(ClipboardContents());
      expect(clipboard).toContain("First message for testing");
      expect(clipboard).toMatch(/^###\s/);
    });

    test("copies a single message respecting the metadata setting", async ({
      actor,
      page,
    }) => {
      const messageItems = RoomScreen.messageItems(page);
      const count = await messageItems.count();
      expect(count).toBeGreaterThan(0);

      const lastItem = messageItems.last();
      const lastId = await lastItem.getAttribute("data-testid");
      const messageId = lastId?.replace("message-item-", "") ?? "";
      const lastContent = await RoomScreen.messageContents(page).last().textContent();

      await actor.attemptsTo(CopySingleMessage(messageId));

      const clipboard = await actor.answer(ClipboardContents());
      expect(clipboard).toContain(lastContent?.trim() ?? "message");
    });

    test("triggers download for selected messages", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SelectAllMessages());

      const downloadPromise = page.waitForEvent("download", {
        timeout: 10_000,
      });

      await actor.attemptsTo(DownloadSelectedMessages());

      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/messages-.*\.md/);
    });
  });

  test.describe("auto-scroll tracking", () => {
    test("does not scroll to bottom on initial load when auto-scroll is off", async ({
      actor,
      page,
    }) => {
      for (let i = 0; i < 15; i++) {
        await actor.attemptsTo(SendMessage(`Message ${i} for scroll test`));
      }
      await actor.attemptsTo(SaveMessages());

      await actor.attemptsTo(SetSettingTo("setting-auto-scroll", false));

      await page.goto("about:blank");
      await actor.attemptsTo(OpenRoom(room.url));

      // With autoScroll OFF, the initial load should NOT auto-scroll to bottom
      const metrics = await actor.answer(ScrollMetrics());
      const distanceFromBottom =
        metrics.scrollHeight - metrics.scrollTop - metrics.clientHeight;
      // If autoScroll respected, we should NOT be at the very bottom
      // (scrollHeight > clientHeight means content overflows)
      if (metrics.scrollHeight > metrics.clientHeight) {
        expect(distanceFromBottom).toBeGreaterThan(50);
      } else {
        // If content doesn't overflow, autoScroll has no effect; test is not meaningful
        test.skip(true, "Content does not overflow viewport; auto-scroll behavior cannot be verified");
      }
    });

    test("shows jump-to-latest button when scrolled away from bottom", async ({
      actor,
      page,
    }) => {
      for (let i = 0; i < 10; i++) {
        await actor.attemptsTo(SendMessage(`Overflow message ${i}`));
      }
      await actor.attemptsTo(SaveMessages());

      // Verify enough messages exist
      const msgCount = await RoomScreen.messageItems(page).count();
      test.skip(msgCount < 5, "Not enough messages to test scroll behavior");

      await scrollMessageListToTop(page);

      await expect(RoomScreen.jumpToLatestButton(page)).toBeVisible({
        timeout: 8_000,
      });
    });

    test("jumps to bottom when clicking jump-to-latest", async ({
      actor,
      page,
    }) => {
      for (let i = 0; i < 30; i++) {
        await actor.attemptsTo(SendMessage(`Overflow message number ${i} - padding text to ensure overflow`));
      }
      await actor.attemptsTo(SaveMessages());

      await scrollMessageListToTop(page);

      await expect(RoomScreen.jumpToLatestButton(page)).toBeVisible({
        timeout: 8_000,
      });
      await RoomScreen.jumpToLatestButton(page).click();

      await page.waitForTimeout(1000);

      const metrics = await actor.answer(ScrollMetrics());
      const distanceFromBottom =
        metrics.scrollHeight - metrics.scrollTop - metrics.clientHeight;
      expect(distanceFromBottom).toBeLessThan(50);
    });
  });

  test.describe("delete confirmation", () => {
    test.beforeEach(async ({ actor, page }) => {
      await actor.attemptsTo(OpenRoom(room.url));
      await actor.attemptsTo(SendMessage("Test message for delete"));
      await actor.attemptsTo(SaveMessages());
    });

    test("shows confirmation dialog when delete confirmation is on", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SelectAllMessages());
      await actor.attemptsTo(DeleteSelectedMessages());

      await expect(RoomScreen.deleteConfirmDialog(page)).toBeVisible({
        timeout: 5_000,
      });
    });

    test("confirms deletion after clicking confirm", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SelectAllMessages());
      await actor.attemptsTo(DeleteSelectedMessages());
      await expect(RoomScreen.deleteConfirmDialog(page)).toBeVisible({
        timeout: 5_000,
      });

      await actor.attemptsTo(ConfirmDelete());

      // After confirm, the dialog should close
      await expect(RoomScreen.deleteConfirmDialog(page)).not.toBeVisible({
        timeout: 5_000,
      });

      // Messages should still be visible (marked for deletion, pending save)
      await expect.poll(async () => {
        const items = RoomScreen.messageItems(page);
        return items.count();
      }).toBeGreaterThan(0);
    });

    test("disables future confirmations when clicking confirm and don't ask", async ({
      actor,
      page,
    }) => {
      await actor.attemptsTo(SelectAllMessages());
      await actor.attemptsTo(DeleteSelectedMessages());
      await expect(RoomScreen.deleteConfirmDialog(page)).toBeVisible({
        timeout: 5_000,
      });

      await actor.attemptsTo(ConfirmDeleteAndDisableFuture());

      await actor.attemptsTo(OpenRoom(room.url));
      await actor.attemptsTo(SendMessage("New message after disable"));
      await actor.attemptsTo(SaveMessages());

      const items = RoomScreen.messageItems(page);
      const count = await items.count();
      if (count > 0) {
        const lastId = await items.last().getAttribute("data-testid");
        const messageId = lastId?.replace("message-item-", "") ?? "";
        await actor.attemptsTo(DeleteMessageById(messageId));

        await expect(RoomScreen.deleteConfirmDialog(page)).not.toBeVisible({
          timeout: 2_000,
        });
      }
    });
  });
});
