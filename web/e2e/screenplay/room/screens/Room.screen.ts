import type { Locator, Page } from "@playwright/test";

import { tCommon, tPattern, tRoom } from "../../support/i18n";

export const RoomScreen = {
  mainArea: (page: Page): Locator =>
    page.locator("main"),

  brandLabel: (page: Page): Locator =>
    page.getByText("Elizabeth", { exact: true }),

  githubProjectLink: (page: Page): Locator =>
    page.getByTestId("github-project-link"),

  topbarSelectionActions: (page: Page): Locator =>
    page.getByTestId("topbar-selection-actions"),

  messageInput: (page: Page): Locator =>
    page.locator(".tiptap-editor-content [contenteditable='true']").first(),

  sourceEditor: (page: Page): Locator =>
    page.locator("textarea").first(),

  sendButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.send") }).first(),

  codeBlockToolbarButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.toolbarCodeBlock") }).first(),

  codeBlockLanguageSelect: (page: Page): Locator =>
    page.getByTestId("code-block-language-select").first(),

  sourceModeButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageInput.toolbarSourceMode") }).first(),

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

  messageCheckboxes: (page: Page): Locator =>
    page.getByTestId(/^message-checkbox-/),

  messageUnsavedBadges: (page: Page): Locator =>
    page.getByTestId(/^message-unsaved-badge-/),

  messageEditedBadges: (page: Page): Locator =>
    page.getByTestId(/^message-edited-badge-/),

  messageEditingBadges: (page: Page): Locator =>
    page.getByTestId(/^message-editing-badge-/),

  messageCountSummary: (page: Page): Locator =>
    page.getByText(tPattern(tRoom("messageList.totalCount"), { count: /\d+/ })),

  messageSelectAllButton: (page: Page): Locator =>
    page.locator("main").getByRole("button", { name: tRoom("messageList.selectAll") }).first(),

  messageInvertSelectionButton: (page: Page): Locator =>
    page.locator("main").getByRole("button", { name: tRoom("messageList.invertSelection") }).first(),

  messageListScroll: (page: Page): Locator =>
    page.getByTestId("message-list-scroll"),

  messageSelectionToolbar: (page: Page): Locator =>
    page.getByTestId("message-selection-toolbar"),

  jumpToLatestButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.scrollToLatest") }),

  mobileChatTab: (page: Page): Locator =>
    page.getByRole("tab", { name: tCommon("mobileTabChat") }),

  mobileBottomTabs: (page: Page): Locator =>
    page.getByTestId("mobile-bottom-tabs"),

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

  roomAddressChangedAlert: (page: Page): Locator =>
    page.locator("div[role='alert']").filter({ hasText: tCommon("roomAddressChanged") }).first(),

  fileSidebar: (page: Page): Locator =>
    page.locator("aside").last(),

  fileUploadButton: (page: Page): Locator =>
    page.locator("aside").last().locator(`button[title='${tRoom("fileManager.uploadFile")}']`),

  fileAddLinkButton: (page: Page): Locator =>
    page.locator("aside").last().locator(`button[title='${tRoom("fileManager.addLink")}']`),

  fileInput: (page: Page): Locator =>
    page.locator("input[type='file']").last(),

  urlUploadUrlInput: (page: Page): Locator =>
    page.getByRole("dialog").locator("#url"),

  urlUploadNameInput: (page: Page): Locator =>
    page.getByRole("dialog").locator("#name"),

  urlUploadDescriptionInput: (page: Page): Locator =>
    page.getByRole("dialog").locator("#description"),

  urlUploadSubmitButton: (page: Page): Locator =>
    page.getByRole("dialog").getByRole("button", {
      name: tRoom("urlUpload.addLink"),
    }),

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

  // Transfer progress panel
  transferProgressPanel: (page: Page): Locator =>
    page.locator("aside").last().locator(".border-b.bg-muted\\/30"),

  transferRows: (page: Page): Locator =>
    page.locator("aside").last().locator(".border-b.bg-muted\\/30 > div"),

  transferCancelButton: (page: Page): Locator =>
    page.locator("aside").last().locator(".border-b.bg-muted\\/30 button[title='Cancel']"),

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

  filePreviewDeleteButton: (page: Page): Locator =>
    page.getByTestId("file-preview-delete"),

  filePreviewIframe: (page: Page): Locator =>
    page.locator("dialog iframe, [role='dialog'] iframe"),

  toast: (page: Page): Locator =>
    page.locator(
      "[data-state='open'][data-swipe-direction], [data-state='open'][data-sonner-toast], [data-state='open'][role='status'], [data-state='open'][role='alert']",
    ).first(),

  closeRoomPasswordError: (page: Page): Locator =>
    page.getByRole("dialog").locator("p.text-destructive, [class*='destructive'] p").first(),

  // Settings dialog switches
  settingIncludeMetadataCopy: (page: Page): Locator =>
    page.getByTestId("setting-include-metadata-copy"),

  settingIncludeMetadataDownload: (page: Page): Locator =>
    page.getByTestId("setting-include-metadata-download"),

  settingDeleteConfirmation: (page: Page): Locator =>
    page.getByTestId("setting-delete-confirmation"),

  settingAutoScroll: (page: Page): Locator =>
    page.getByTestId("setting-auto-scroll"),

  settingDesktopNotifications: (page: Page): Locator =>
    page.getByTestId("setting-desktop-notifications"),

  settingDesktopNotificationShowContent: (page: Page): Locator =>
    page.getByTestId("setting-desktop-notification-show-content"),

  settingDesktopNotificationType: (
    page: Page,
    kind: "message" | "file" | "link",
    action: "created" | "updated" | "deleted",
  ): Locator =>
    page.getByTestId(`setting-desktop-notification-${kind}-${action}`),

  settingsDialog: (page: Page): Locator =>
    page.getByTestId("settings-dialog"),

  settingsDialogScroll: (page: Page): Locator =>
    page.getByTestId("settings-dialog-scroll"),

  settingsTab: (page: Page, tab: string): Locator =>
    page.getByTestId(`settings-tab-${tab}`),

  settingsTabPanel: (page: Page, tab: string): Locator =>
    page.getByTestId(`settings-tab-${tab}-panel`),

  settingsNotificationAccordion: (page: Page): Locator =>
    page.getByTestId("settings-notification-accordion"),

  settingsNotificationKindTrigger: (page: Page, kind: string): Locator =>
    page.getByTestId(`settings-notification-${kind}-trigger`),

  // Delete confirmation dialogs
  fileDeleteConfirmDialog: (page: Page): Locator =>
    page.getByTestId("file-delete-confirm-dialog"),

  fileDeleteConfirmButton: (page: Page): Locator =>
    RoomScreen.fileDeleteConfirmDialog(page).getByRole("button", {
      name: tRoom("fileDeleteConfirm.confirm"),
    }),

  fileDeleteCancelButton: (page: Page): Locator =>
    RoomScreen.fileDeleteConfirmDialog(page).getByRole("button", {
      name: tRoom("fileDeleteConfirm.cancel"),
    }),

  deleteConfirmDialog: (page: Page): Locator =>
    page.locator('[data-testid="delete-confirm-dialog"], [role="alertdialog"]').first(),

  deleteConfirmButton: (page: Page): Locator =>
    RoomScreen.deleteConfirmDialog(page).getByRole("button", {
      name: /^(?!.*(?:don't|不再)).*(?:confirm|确认)$/i,
    }).first(),

  deleteConfirmAndDisableButton: (page: Page): Locator =>
    RoomScreen.deleteConfirmDialog(page).getByRole("button", {
      name: /(?:don.t.*ask|不再|confirm.*don.t)/i,
    }).first(),

  deleteCancelButton: (page: Page): Locator =>
    RoomScreen.deleteConfirmDialog(page).getByRole("button", {
      name: /cancel|取消/i,
    }).first(),

  // Message meta row
  messageMeta: (page: Page, messageId: string): Locator =>
    page.getByTestId(`message-meta-${messageId}`),
};
