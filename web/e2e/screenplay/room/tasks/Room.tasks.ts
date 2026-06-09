import { Task, the } from "@serenity-js/core";
import { Navigate } from "@serenity-js/web";

import {
  CancelDialog,
  CancelFirstTransfer,
  ClickCopyMessages,
  ClickDeleteMessages,
  ClickDownloadMessages,
  ClickMessageCopyButton,
  ClickMessageDeleteButton,
  CloseSettings,
  ConfirmDeleteAction,
  ConfirmDeleteAndDisable,
  OpenSettings,
  ClickSaveMessages,
  ClickSend,
  ClickFilePreviewCopyLink,
  ClickFilePreviewCopyMarkdown,
  ClickFilePreviewDownload,
  ClickFilePreviewInsertToEditor,
  ConfirmPhysicalClose,
  DeleteFileNamed,
  EditLatestMessage,
  EnterMessage,
  EnterRoomPassword,
  OpenCloseRoomDialog,
  OpenFilePreviewNamed,
  ResizeViewport,
  SaveRoomConfiguration,
  ScrollMessageListToBottom,
  ScrollMessageListToTop,
  ToggleSetting,
  SetSettingState,
  SelectRoomExpiry,
  SetPermissionState,
  SetRoomMaxViews,
  SetRoomPassword,
  UploadFiles,
  VerifyCloseRoomPassword,
  WaitForRoomToBeReady,
  WaitForSavingToComplete,
} from "../interactions/Room.interactions";
import type { UploadableFile } from "../../support/test-data";

export const OpenRoom = (url: string) =>
  Task.where(
    the`#actor opens the room at ${url}`,
    Navigate.to(url),
    WaitForRoomToBeReady(),
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

export const ConfigureRoom = (config: {
  expiry?: string;
  password?: string;
  maxViews?: number;
}) =>
  Task.where(
    the`#actor updates the room configuration`,
    ...(config.expiry ? [SelectRoomExpiry(config.expiry)] : []),
    ...(config.password !== undefined ? [SetRoomPassword(config.password)] : []),
    ...(config.maxViews !== undefined ? [SetRoomMaxViews(config.maxViews)] : []),
    SaveRoomConfiguration(),
  );

export const SetRoomPermissions = (permissions: Partial<Record<"read" | "edit" | "share" | "delete", boolean>>) =>
  {
    const entries = Object.entries(permissions)
      .filter(([, desired]) => desired !== undefined) as Array<
        ["read" | "edit" | "share" | "delete", boolean]
      >;

    const enableOrder: Array<"read" | "edit" | "share" | "delete"> = [
      "read",
      "edit",
      "share",
      "delete",
    ];
    const disableOrder: Array<"read" | "edit" | "share" | "delete"> = [
      "delete",
      "share",
      "edit",
      "read",
    ];

    const ordered = [
      ...disableOrder
        .filter((label) => entries.some(([entry, desired]) => entry === label && desired === false))
        .map((label) => [label, false] as const),
      ...enableOrder
        .filter((label) => entries.some(([entry, desired]) => entry === label && desired === true))
        .map((label) => [label, true] as const),
    ];

    return Task.where(
      the`#actor updates room permissions`,
      ...ordered.map(([label, desired]) => SetPermissionState(label, desired)),
      SaveRoomConfiguration(),
    );
  };

export const UploadRoomFiles = (...files: UploadableFile[]) =>
  Task.where(
    the`#actor uploads room files`,
    UploadFiles(...files),
  );

export const DeleteRoomFile = (name: string) =>
  Task.where(
    the`#actor deletes the room file ${name}`,
    DeleteFileNamed(name),
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

export const CancelTransfer = () =>
  Task.where(
    the`#actor cancels the first active transfer`,
    CancelFirstTransfer(),
  );

export const CloseRoomWithoutPassword = () =>
  Task.where(
    the`#actor closes the room without a password challenge`,
    OpenCloseRoomDialog(),
    ConfirmPhysicalClose(),
  );

export const CloseRoomWithPassword = (password: string) =>
  Task.where(
    the`#actor closes the room using a password challenge`,
    OpenCloseRoomDialog(),
    VerifyCloseRoomPassword(password),
    ConfirmPhysicalClose(),
  );

export const CancelRoomClosure = (password?: string) =>
  Task.where(
    the`#actor cancels closing the room`,
    OpenCloseRoomDialog(),
    ...(password ? [VerifyCloseRoomPassword(password)] : []),
    CancelDialog(),
  );

export const UnlockProtectedRoom = (password: string) =>
  Task.where(
    the`#actor unlocks the protected room`,
    EnterRoomPassword(password),
    WaitForRoomToBeReady(),
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

export const ScrollMessagesUp = () =>
  Task.where(
    the`#actor scrolls away from the latest message`,
    ScrollMessageListToTop(),
  );

export const ScrollMessagesDown = () =>
  Task.where(
    the`#actor scrolls back to the latest message`,
    ScrollMessageListToBottom(),
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

export const EnableSetting = (testid: string) =>
  Task.where(
    the`#actor enables the setting`,
    OpenSettings(),
    ToggleSetting(testid),
    CloseSettings(),
  );

export const SetSettingTo = (testid: string, desired: boolean) =>
  Task.where(
    the`#actor sets the setting to ${desired}`,
    OpenSettings(),
    SetSettingState(testid, desired),
    CloseSettings(),
  );

export const ToggleSettingInOpenDialog = (testid: string) =>
  Task.where(
    the`#actor toggles a setting in the open settings dialog`,
    ToggleSetting(testid),
  );
