import { Interaction, the } from "@serenity-js/core";

import { nativePageFor } from "../../support/actor-page";
import { tRoom } from "../../support/i18n";
import type { UploadableFile } from "../../support/test-data";
import { RoomScreen } from "../screens/Room.screen";

export const WaitForRoomToBeReady = () =>
  Interaction.where(the`#actor waits for the room UI to be ready`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageInput(page).waitFor({ state: "visible", timeout: 30_000 });
  });

export const EnterMessage = (content: string) =>
  Interaction.where(the`#actor enters the message ${content}`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageInput(page).fill(content);
  });

export const ClickSend = () =>
  Interaction.where(the`#actor sends the current message`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.sendButton(page).click();
  });

export const ClickSaveMessages = () =>
  Interaction.where(the`#actor saves all pending messages`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.saveMessagesButton(page).click();
  });

export const ResizeViewport = (width: number, height: number) =>
  Interaction.where(the`#actor resizes the viewport to ${width}x${height}`, async (actor) => {
    const page = await nativePageFor(actor);
    await page.setViewportSize({ width, height });
  });

export const SelectRoomExpiry = (optionLabel: string) =>
  Interaction.where(the`#actor selects the room expiry ${optionLabel}`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.expirySelect(page).click();
    await page.getByRole("option", { name: optionLabel }).click();
  });

export const SetRoomPassword = (password: string) =>
  Interaction.where(the`#actor sets the room password`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.roomPasswordInput(page).fill(password);
  });

export const SetRoomMaxViews = (count: number) =>
  Interaction.where(the`#actor sets the maximum room views to ${count}`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.maxViewsInput(page).fill(String(count));
  });

export const SaveRoomConfiguration = () =>
  Interaction.where(the`#actor saves the room configuration`, async (actor) => {
    const page = await nativePageFor(actor);
    const saveButton = RoomScreen.saveRoomConfigButton(page);
    if (await saveButton.isDisabled()) {
      throw new Error("Room configuration save button is disabled before saving");
    }

    const saveRequest = page.waitForResponse((response) => {
      const url = response.url();
      const method = response.request().method();
      return url.includes("/api/v1/rooms/") &&
        (
          (method === "PUT" && url.includes("/settings")) ||
          (method === "POST" && url.includes("/permissions"))
        );
    }, { timeout: 10_000 });

    await saveButton.click();

    const response = await saveRequest;
    if (!response.ok()) {
      throw new Error(
        `Saving room configuration failed: ${response.status()} ${response.statusText()}`,
      );
    }

    try {
      await RoomScreen.toast(page).waitFor({ state: "visible", timeout: 5_000 });
    } catch {
      if (!(await saveButton.isDisabled())) {
        throw new Error("Room configuration save completed without a visible success signal");
      }
    }
  });

export const SetPermissionState = (
  label: "read" | "edit" | "share" | "delete",
  desired: boolean,
) =>
  Interaction.where(the`#actor sets the ${label} permission to ${desired}`, async (actor) => {
    const page = await nativePageFor(actor);
    const button = RoomScreen.permissionButton(
      page,
      tRoom(`config.permissions.labels.${label}`),
    );
    const current = await button.getAttribute("aria-pressed");

    if ((current === "true") !== desired) {
      await button.click();
    }
  });

export const SelectAllMessages = () =>
  Interaction.where(the`#actor selects all messages`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageSelectAllButton(page).click();
  });

export const InvertMessageSelection = () =>
  Interaction.where(the`#actor inverts the selected messages`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageInvertSelectionButton(page).click();
  });

export const SelectAllFiles = () =>
  Interaction.where(the`#actor selects all files`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.fileSelectAllButton(page).click();
  });

export const InvertFileSelection = () =>
  Interaction.where(the`#actor inverts the selected files`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.fileInvertSelectionButton(page).click();
  });

export const UploadFiles = (...files: UploadableFile[]) =>
  Interaction.where(the`#actor uploads ${files.length} file(s)`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.fileInput(page).setInputFiles(files);
  });

export const DeleteFileNamed = (name: string) =>
  Interaction.where(the`#actor deletes the file ${name}`, async (actor) => {
    const page = await nativePageFor(actor);
    const fileCard = RoomScreen.fileCards(page).filter({
      has: page.getByText(name, { exact: true }),
    }).first();
    await fileCard.hover();
    await fileCard.getByRole("button", { name: tRoom("fileCard.deleteFile") }).click();
  });

export const OpenFilePreviewNamed = (name: string) =>
  Interaction.where(the`#actor opens the preview for ${name}`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.fileCards(page).filter({
      has: page.getByText(name, { exact: true }),
    }).first().click();
  });

export const ClickFilePreviewDownload = () =>
  Interaction.where(the`#actor downloads the previewed file`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.filePreviewDownloadButton(page).click();
  });

export const ClickFilePreviewCopyLink = () =>
  Interaction.where(the`#actor copies the public download link from the preview`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.filePreviewCopyLinkButton(page).click();
  });

export const ClickFilePreviewCopyMarkdown = () =>
  Interaction.where(the`#actor copies the markdown reference from the preview`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.filePreviewCopyMarkdownButton(page).click();
  });

export const ClickFilePreviewInsertToEditor = () =>
  Interaction.where(the`#actor inserts the preview markdown into the editor`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.filePreviewInsertToEditorButton(page).click();
  });

export const OpenCloseRoomDialog = () =>
  Interaction.where(the`#actor opens the close room dialog`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.closeRoomButton(page).click();
  });

export const VerifyCloseRoomPassword = (password: string) =>
  Interaction.where(the`#actor verifies the close room password`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.closeRoomPasswordInput(page).fill(password);
    await RoomScreen.closeRoomNextButton(page).click();
  });

export const ConfirmPhysicalClose = () =>
  Interaction.where(the`#actor confirms the physical room closure`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.closeRoomConfirmButton(page).click();
  });

export const CancelDialog = () =>
  Interaction.where(the`#actor cancels the open dialog`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.closeRoomCancelButton(page).click();
  });

export const EnterRoomPassword = (password: string) =>
  Interaction.where(the`#actor enters the room password`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.passwordDialogInput(page).fill(password);
    await RoomScreen.passwordDialogEnterRoomButton(page).click();
  });

export const EditLatestMessage = (content: string) =>
  Interaction.where(the`#actor edits the latest message`, async (actor) => {
    const page = await nativePageFor(actor);
    const latestMessage = RoomScreen.messageItems(page).last();
    await latestMessage.hover();
    await latestMessage.getByRole("button", { name: tRoom("messageBubble.edit") }).click();
    await RoomScreen.messageInput(page).fill(content);
    await RoomScreen.sendButton(page).click();
  });

export const ScrollMessageListToTop = () =>
  Interaction.where(the`#actor scrolls the message list to the top`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageListScroll(page).evaluate((element) => {
      const viewport = element.querySelector("[data-radix-scroll-area-viewport]") as HTMLDivElement | null;
      if (viewport) {
        viewport.scrollTop = 0;
        viewport.dispatchEvent(new Event("scroll"));
      }
    });
  });

export const ScrollMessageListToBottom = () =>
  Interaction.where(the`#actor scrolls the message list to the bottom`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.messageListScroll(page).evaluate((element) => {
      const viewport = element.querySelector("[data-radix-scroll-area-viewport]") as HTMLDivElement | null;
      if (viewport) {
        viewport.scrollTop = viewport.scrollHeight;
      }
    });
  });
export const OpenSettings = () =>
  Interaction.where(the`#actor opens the settings dialog`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.settingsButton(page).click();
    await page.getByRole("dialog").filter({ hasText: /settings|设置/i }).first().waitFor({ state: "visible" });
  });

export const CloseSettings = () =>
  Interaction.where(the`#actor closes the settings dialog`, async (actor) => {
    const page = await nativePageFor(actor);
    const dialog = RoomScreen.settingsDialog(page);
    const closeButton = dialog.locator('[data-slot="dialog-close"]').first();

    if (await closeButton.isVisible().catch(() => false)) {
      await closeButton.click();
      await dialog.waitFor({ state: "hidden", timeout: 5_000 });
      return;
    }

    await page.keyboard.press("Escape");
    if (await dialog.isVisible({ timeout: 500 }).catch(() => false)) {
      await page.mouse.click(10, 10);
      await dialog.waitFor({ state: "hidden", timeout: 5_000 }).catch(() => {});
    }
  });

export const ToggleSetting = (testid: string) =>
  Interaction.where(the`#actor toggles the setting ${testid}`, async (actor) => {
    const page = await nativePageFor(actor);
    await page.getByTestId(testid).click();
  });

export const SetSettingState = (testid: string, desired: boolean) =>
  Interaction.where(
    the`#actor sets the setting ${testid} to ${desired}`,
    async (actor) => {
      const page = await nativePageFor(actor);
      const toggle = page.getByTestId(testid);
      const current = await toggle.getAttribute("aria-checked");

      if ((current === "true") !== desired) {
        await toggle.click();
      }
    },
  );

export const ClickCopyMessages = () =>
  Interaction.where(the`#actor copies the selected messages`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.copyMessagesButton(page).click();
  });

export const ClickDownloadMessages = () =>
  Interaction.where(the`#actor downloads the selected messages`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.downloadMessagesButton(page).click();
  });

export const ClickDeleteMessages = () =>
  Interaction.where(the`#actor clicks the delete messages button`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.deleteMessagesButton(page).click();
  });

export const ConfirmDeleteAction = () =>
  Interaction.where(the`#actor confirms the delete action`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.deleteConfirmButton(page).click();
  });

export const ConfirmDeleteAndDisable = () =>
  Interaction.where(the`#actor confirms delete and disables future confirmations`, async (actor) => {
    const page = await nativePageFor(actor);
    await RoomScreen.deleteConfirmAndDisableButton(page).click();
  });

export const HoverMessage = (messageId: string) =>
  Interaction.where(the`#actor hovers over the message ${messageId}`, async (actor) => {
    const page = await nativePageFor(actor);
    await page.getByTestId(`message-item-${messageId}`).hover();
  });

export const ClickMessageCopyButton = (messageId: string) =>
  Interaction.where(the`#actor copies the message ${messageId}`, async (actor) => {
    const page = await nativePageFor(actor);
    const item = page.getByTestId(`message-item-${messageId}`);
    await item.hover();
    await item.getByRole("button", { name: /copy|复制/i }).click();
  });

export const ClickMessageDeleteButton = (messageId: string) =>
  Interaction.where(the`#actor deletes the message ${messageId}`, async (actor) => {
    const page = await nativePageFor(actor);
    const item = page.getByTestId(`message-item-${messageId}`);
    await item.hover();
    await item.getByRole("button", { name: /delete|删除/i }).click();
  });
