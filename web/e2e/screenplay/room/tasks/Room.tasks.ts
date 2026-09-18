import { Interaction, Task, the } from "@serenity-js/core";
import { Navigate } from "@serenity-js/web";

import { nativePageFor } from "../../support/actor-page";

import {
  CancelFileDeleteAction,
  ClickFilePreviewDelete,
  ClickCopyMessages,
  ClickDeleteMessages,
  ClickDownloadMessages,
  ClickMessageCopyButton,
  ClickMessageDeleteButton,
  CloseFilePreviewDialog,
  CloseSettings,
  ConfirmDeleteAction,
  ConfirmDeleteAndDisable,
  ConfirmFileDeleteAction,
  ConfigureFileDownloadPolicy,
  OpenSettings,
  ClickSaveMessages,
  ClickSend,
  ClickFilePreviewCopyLink,
  ClickFilePreviewCopyMarkdown,
  ClickFilePreviewDownload,
  ClickFilePreviewInsertToEditor,
  AddLinkToRoom,
  DeleteFileNamed,
  DropFileOntoEditor,
  EditLatestMessage,
  EnterMessage,
  EnterRoomPassword,
  OpenFilePreviewNamed,
  OpenIdentityRedeemDialog,
  PasteFileIntoEditor,
  PasteTextIntoEditor,
  RedeemAccessCode,
  RedeemIdentityCode,
  ResizeViewport,
  SaveRoomConfiguration,
  SetSettingState,
  SelectRoomDuration,
  SelectRoomDefaultRole,
  SetRoomMaxSize,
  SetRoomMaxViews,
  SetRoomPassword,
  UploadFiles,
  WaitForRoomToBeReady,
  WaitForSavingToComplete,
  type DownloadPolicyInput,
} from "../interactions/Room.interactions";
import { RoomScreen } from "../screens/Room.screen";
import type { UploadableFile } from "../../support/test-data";

export const OpenRoom = (url: string) =>
  Task.where(
    the`#actor opens the room at ${url}`,
    Navigate.to(url),
    WaitForRoomToBeReady(),
  );

export const OpenUnprovisionedRoom = (url: string) =>
  Task.where(
    the`#actor opens an unprovisioned room at ${url}`,
    Navigate.to(url),
    Interaction.where(
      the`#actor waits for the one-time identity code disclosure`,
      async (actor) => {
        const page = await nativePageFor(actor);
        await RoomScreen.identityCodeDisclosure(page).waitFor({
          state: "visible",
          timeout: 30_000,
        });
      },
    ),
  );

export const VisitRoomUrl = (url: string) =>
  Task.where(
    the`#actor visits ${url} without waiting for the room UI`,
    Navigate.to(url),
  );

export const SendMessage = (content: string) =>
  Task.where(
    the`#actor sends the message ${content}`,
    EnterMessage(content),
    ClickSend(),
  );

export const SaveMessages = () =>
  Task.where(
    the`#actor saves the current messages`,
    ClickSaveMessages(),
    WaitForSavingToComplete(),
  );

export const TrySaveMessages = () =>
  Task.where(
    the`#actor tries to save the current messages`,
    ClickSaveMessages(),
  );

export const SendCurrentDraft = () =>
  Task.where(
    the`#actor sends the current draft`,
    ClickSend(),
  );

export const ConfigureRoom = (config: {
  durationSeconds?: number;
  password?: string;
  maxViews?: number;
  maxSizeBytes?: number;
  defaultRoleKey?: string;
}) =>
  Task.where(
    the`#actor updates the room configuration`,
    ...(config.durationSeconds !== undefined
      ? [SelectRoomDuration(config.durationSeconds)]
      : []),
    ...(config.password !== undefined ? [SetRoomPassword(config.password)] : []),
    ...(config.maxViews !== undefined ? [SetRoomMaxViews(config.maxViews)] : []),
    ...(config.maxSizeBytes !== undefined ? [SetRoomMaxSize(config.maxSizeBytes)] : []),
    ...(config.defaultRoleKey !== undefined
      ? [SelectRoomDefaultRole(config.defaultRoleKey)]
      : []),
    SaveRoomConfiguration(),
  );

export const UploadRoomFiles = (...files: UploadableFile[]) =>
  Task.where(
    the`#actor uploads room files`,
    UploadFiles(...files),
  );

export const AddRoomLink = (data: {
  urlInput: string;
  name: string;
  description?: string;
}) =>
  Task.where(
    the`#actor adds a room link`,
    AddLinkToRoom(data),
  );

export const DeleteRoomFile = (name: string) =>
  Task.where(
    the`#actor deletes the room file ${name}`,
    DeleteFileNamed(name),
  );

export const ConfirmFileDelete = () =>
  Task.where(
    the`#actor confirms the file delete`,
    ConfirmFileDeleteAction(),
  );

export const CancelFileDelete = () =>
  Task.where(
    the`#actor cancels the file delete`,
    CancelFileDeleteAction(),
  );

export const PreviewRoomFile = (name: string) =>
  Task.where(
    the`#actor previews the room file ${name}`,
    OpenFilePreviewNamed(name),
  );

export const DownloadPreviewedRoomFile = () =>
  Task.where(
    the`#actor downloads the previewed room file`,
    ClickFilePreviewDownload(),
  );

export const CopyPreviewRoomFileLink = () =>
  Task.where(
    the`#actor copies the previewed room file link`,
    ClickFilePreviewCopyLink(),
  );

export const CopyPreviewRoomFileMarkdown = () =>
  Task.where(
    the`#actor copies the previewed room file markdown`,
    ClickFilePreviewCopyMarkdown(),
  );

export const InsertPreviewRoomFileMarkdown = () =>
  Task.where(
    the`#actor inserts the previewed room file markdown into the editor`,
    ClickFilePreviewInsertToEditor(),
  );

export const DeletePreviewedRoomFile = () =>
  Task.where(
    the`#actor deletes the previewed room file`,
    ClickFilePreviewDelete(),
  );

export const UnlockProtectedRoom = (password: string) =>
  Task.where(
    the`#actor unlocks the protected room`,
    EnterRoomPassword(password),
    WaitForRoomToBeReady(),
  );

export const EnterRoomAfterDisclosure = () =>
  Task.where(
    the`#actor enters the room after the identity code disclosure`,
    Interaction.where(the`#actor acknowledges the disclosed identity code`, async (actor) => {
      const page = await nativePageFor(actor);
      await RoomScreen.enterRoomAfterDisclosure(page).click();
    }),
    WaitForRoomToBeReady(),
  );

export const RedeemIdentityCodeInRoom = (code: string) =>
  Task.where(
    the`#actor upgrades the session with the identity code ${code}`,
    OpenIdentityRedeemDialog(),
    RedeemIdentityCode(code),
    Interaction.where(the`#actor waits for the reloaded session`, async (actor) => {
      const page = await nativePageFor(actor);
      await page.waitForLoadState("load").catch(() => {});
      await WaitForRoomToBeReady().performAs(actor);
    }),
  );

export const UpdateLatestMessage = (content: string) =>
  Task.where(
    the`#actor updates the latest message`,
    EditLatestMessage(content),
  );

export const SwitchToMobileViewport = () =>
  Task.where(
    the`#actor switches to the mobile viewport`,
    ResizeViewport(390, 844),
  );

export const SwitchToShortMobileViewport = () =>
  Task.where(
    the`#actor switches to a short mobile viewport`,
    ResizeViewport(390, 560),
  );

export const CopySelectedMessages = () =>
  Task.where(
    the`#actor copies the selected messages to clipboard`,
    ClickCopyMessages(),
  );

export const DownloadSelectedMessages = () =>
  Task.where(
    the`#actor downloads the selected messages`,
    ClickDownloadMessages(),
  );

export const DeleteSelectedMessages = () =>
  Task.where(
    the`#actor deletes the selected messages`,
    ClickDeleteMessages(),
  );

export const DeleteMessageById = (messageId: string) =>
  Task.where(
    the`#actor deletes the message by id`,
    ClickMessageDeleteButton(messageId),
  );

export const ConfirmDelete = () =>
  Task.where(
    the`#actor confirms the delete`,
    ConfirmDeleteAction(),
  );

export const ConfirmDeleteAndDisableFuture = () =>
  Task.where(
    the`#actor confirms delete and disables future confirmations`,
    ConfirmDeleteAndDisable(),
  );

export const CopySingleMessage = (messageId: string) =>
  Task.where(
    the`#actor copies a single message`,
    ClickMessageCopyButton(messageId),
  );

export const SetSettingTo = (testid: string, desired: boolean) =>
  Task.where(
    the`#actor sets the setting to ${desired}`,
    OpenSettings(),
    SetSettingState(testid, desired),
    CloseSettings(),
  );

export const ConfigureDownloadPolicy = (
  fileName: string,
  policy: DownloadPolicyInput,
) =>
  Task.where(
    the`#actor configures the download policy of ${fileName}`,
    ConfigureFileDownloadPolicy(fileName, policy),
  );

export const RedeemFileAccessCode = (code: string) =>
  Task.where(
    the`#actor redeems the file access code ${code}`,
    RedeemAccessCode(code),
  );

export const CloseFilePreview = () =>
  Task.where(
    the`#actor closes the file preview`,
    CloseFilePreviewDialog(),
  );

export const PasteIntoComposer = (text: string) =>
  Task.where(
    the`#actor pastes text into the message composer`,
    PasteTextIntoEditor(text),
  );

export const PasteFileIntoComposer = (file: UploadableFile) =>
  Task.where(
    the`#actor pastes ${file.name} into the message composer`,
    PasteFileIntoEditor(file),
  );

export const DropFileIntoComposer = (file: UploadableFile) =>
  Task.where(
    the`#actor drops ${file.name} into the message composer`,
    DropFileOntoEditor(file),
  );
