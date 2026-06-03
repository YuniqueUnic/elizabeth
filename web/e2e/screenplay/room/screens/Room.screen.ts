import type { Locator, Page } from "@playwright/test";

import { tCommon, tPattern, tRoom } from "../../support/i18n";

export const RoomScreen = {
  mainArea: (page: Page): Locator =>
    page.locator("main"),

  messageInput: (page: Page): Locator =>
    page.locator(".tiptap-editor-content [contenteditable='true']").first(),

  sendButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.send") }).first(),

  expandEditorButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.expandEditor") }),

  saveMessagesButton: (page: Page): Locator =>
    page.getByTestId("save-messages-btn"),

  copyMessagesButton: (page: Page): Locator =>
    page.getByTestId("copy-messages-btn"),

  downloadMessagesButton: (page: Page): Locator =>
    page.getByTestId("download-messages-btn"),

  deleteMessagesButton: (page: Page): Locator =>
    page.getByTestId("delete-messages-btn"),

  helpButton: (page: Page): Locator =>
    page.getByTestId("help-btn"),

  settingsButton: (page: Page): Locator =>
    page.getByTestId("settings-btn"),

  messageItems: (page: Page): Locator =>
    page.getByTestId(/^message-item-/),

  messageContents: (page: Page): Locator =>
    page.getByTestId(/^message-content-/),

  messageUnsavedBadges: (page: Page): Locator =>
    page.getByTestId(/^message-unsaved-badge-/),

  messageEditedBadges: (page: Page): Locator =>
    page.getByTestId(/^message-edited-badge-/),

  messageEditingBadges: (page: Page): Locator =>
    page.getByTestId(/^message-editing-badge-/),

  messageCountSummary: (page: Page): Locator =>
    page.getByText(tPattern(tRoom("messageList.totalCount"), { count: /\d+/ })),

  messageSelectAllButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.selectAll") }),

  messageInvertSelectionButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.invertSelection") }),

  messageListScroll: (page: Page): Locator =>
    page.getByTestId("message-list-scroll"),

  jumpToLatestButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.scrollToLatest") }),

  editorCancelButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.cancel") }).last(),

  roomConfigRoot: (page: Page): Locator =>
    page.locator("aside").first(),

  roomConfigTitle: (page: Page): Locator =>
    page.locator("aside").first().getByText(tRoom("config.title")),

  expirySelect: (page: Page): Locator =>
    page.getByRole("combobox").first(),

  roomPasswordInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#password"),

  maxViewsInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#max-views"),

  saveRoomConfigButton: (page: Page): Locator =>
    page.locator("aside").first().getByRole("button", {
      name: tRoom("config.save.saveConfig"),
    }),

  resetRoomConfigButton: (page: Page): Locator =>
    page.locator("aside").first().getByRole("button", { name: tRoom("config.cancel") }),

  permissionButton: (page: Page, label: string): Locator =>
    page.locator("aside").first().getByRole("button", { name: label }).first(),

  capacityInfo: (page: Page): Locator =>
    page.locator("aside").first().getByText(tRoom("capacity.title")).locator(".."),

  shareLinkButton: (page: Page): Locator =>
    page.locator("aside").first().getByRole("button", { name: tRoom("sharing.getLink") }).first(),

  shareDownloadQrButton: (page: Page): Locator =>
    page.locator("aside").first().getByRole("button", { name: tRoom("sharing.download") }).first(),

  qrCodeImage: (page: Page): Locator =>
    page.locator("aside").first().locator('img[alt="Room QR Code"]'),

  closeRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("closeRoom.button") }).first(),

  dialog: (page: Page): Locator =>
    page.getByRole("dialog"),

  closeRoomPasswordInput: (page: Page): Locator =>
    page.locator("#close-room-password"),

  closeRoomNextButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("closeRoom.nextStep") }),

  closeRoomConfirmButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("closeRoom.confirmPhysicalClose") }),

  closeRoomCancelButton: (page: Page): Locator =>
    page.getByRole("dialog").getByRole("button", { name: tRoom("closeRoom.cancel") }).first(),

  passwordDialogInput: (page: Page): Locator =>
    page.getByRole("dialog").locator("#password").first(),

  passwordDialogEnterRoomButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("passwordDialog.enterRoom") }),

  passwordDialogError: (page: Page): Locator =>
    page.getByRole("dialog").getByRole("alert"),

  alert: (page: Page): Locator =>
    page.locator("div[role='alert'][data-slot='alert']").first(),

  fileSidebar: (page: Page): Locator =>
    page.locator("aside").last(),

  fileUploadButton: (page: Page): Locator =>
    page.locator("aside").last().locator(`button[title='${tRoom("fileManager.uploadFile")}']`),

  fileInput: (page: Page): Locator =>
    page.locator("input[type='file']").last(),

  fileEmptyState: (page: Page): Locator =>
    page.getByText(tRoom("fileListView.empty")),

  fileCards: (page: Page): Locator =>
    page.locator("div.group.relative.flex.items-center.gap-3.rounded-lg.border"),

  fileCheckboxes: (page: Page): Locator =>
    page.locator("div.group.relative.flex.items-center.gap-3.rounded-lg.border [role='checkbox']"),

  fileNames: (page: Page): Locator =>
    page.locator("div.group.relative.flex.items-center.gap-3.rounded-lg.border p.text-sm.font-medium"),

  fileUploadZone: (page: Page): Locator =>
    page.getByText(tRoom("fileUploadZone.dragOrClick")),

  fileSelectAllButton: (page: Page): Locator =>
    page.locator("aside").last().getByRole("button", { name: tRoom("fileManager.selectAll") }),

  fileInvertSelectionButton: (page: Page): Locator =>
    page.locator("aside").last().getByRole("button", { name: tRoom("fileManager.invertSelection") }),

  filePreviewDialog: (page: Page): Locator =>
    page.getByRole("dialog"),

  filePreviewTitle: (page: Page): Locator =>
    page.getByRole("dialog").locator(".truncate.font-semibold"),

  filePreviewDownloadButton: (page: Page): Locator =>
    page.getByTestId("file-preview-download"),

  filePreviewCopyLinkButton: (page: Page): Locator =>
    page.getByTestId("file-preview-copy-link"),

  filePreviewCopyMarkdownButton: (page: Page): Locator =>
    page.getByTestId("file-preview-copy-markdown"),

  filePreviewInsertToEditorButton: (page: Page): Locator =>
    page.getByTestId("file-preview-insert-markdown"),

  filePreviewIframe: (page: Page): Locator =>
    page.locator("dialog iframe, [role='dialog'] iframe"),

  toast: (page: Page): Locator =>
    page.locator(
      "[data-state='open'][data-swipe-direction], [data-state='open'][data-sonner-toast], [data-state='open'][role='status'], [data-state='open'][role='alert']",
    ).first(),
};
