import { Interaction, the } from "@serenity-js/core";

import { nativePageFor } from "../../support/actor-page";
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
    await RoomScreen.saveRoomConfigButton(page).click();
  });

export const SetPermissionState = (
  label: "预览" | "编辑" | "分享" | "删除",
  desired: boolean,
) =>
  Interaction.where(the`#actor sets the ${label} permission to ${desired}`, async (actor) => {
    const page = await nativePageFor(actor);
    const button = RoomScreen.permissionButton(page, label);
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
    await fileCard.getByRole("button", { name: /删除/ }).click();
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
    await latestMessage.getByRole("button", { name: "编辑" }).click();
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
