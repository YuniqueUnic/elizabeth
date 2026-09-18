import type { Locator, Page } from "@playwright/test";

import { tCommon, tRoom } from "../../support/i18n";

export const RoomScreen = {
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

  messageSelectAllButton: (page: Page): Locator =>
    page.locator("main").getByRole("button", { name: tRoom("messageList.selectAll") }).first(),

  messageListScroll: (page: Page): Locator =>
    page.getByTestId("message-list-scroll"),

  messageListViewport: (page: Page): Locator =>
    page.getByTestId("message-list-scroll").locator(
      "[data-radix-scroll-area-viewport]",
    ),

  messageSelectionToolbar: (page: Page): Locator =>
    page.getByTestId("message-selection-toolbar"),

  jumpToLatestButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.scrollToLatest") }),

  mobileChatTab: (page: Page): Locator =>
    page.getByRole("tab", { name: tCommon("mobileTabChat") }),

  mobileSettingsTab: (page: Page): Locator =>
    page.getByRole("tab", { name: tCommon("mobileTabSettings") }),

  mobileFilesTab: (page: Page): Locator =>
    page.getByRole("tab", { name: tCommon("mobileTabFiles") }),

  loadOlderMessagesButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("messageList.loadOlder") }),

  mobileBottomTabs: (page: Page): Locator =>
    page.getByTestId("mobile-bottom-tabs"),

  leftSidebar: (page: Page): Locator =>
    page.getByTestId("left-sidebar"),

  leftSidebarCollapseButton: (page: Page): Locator =>
    page.getByTestId("left-sidebar-collapse"),

  leftSidebarExpandButton: (page: Page): Locator =>
    page.getByTestId("left-sidebar-expand"),

  leftSidebarCollapsedRail: (page: Page): Locator =>
    page.getByTestId("left-sidebar-collapsed-rail"),

  durationSelect: (page: Page): Locator =>
    page.getByTestId("room-duration-select"),

  durationOption: (page: Page, ageSeconds: number): Locator =>
    page.getByTestId(`room-duration-option-${ageSeconds}`),

  roomExpiryHint: (page: Page): Locator =>
    page.getByTestId("room-expiry-hint"),

  roomPasswordInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#room-password"),

  maxViewsInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#room-max-views"),

  maxSizeInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#room-max-size"),

  defaultRoleSelect: (page: Page): Locator =>
    page.getByTestId("room-default-role-select"),

  defaultRoleOption: (page: Page, roleKey: string): Locator =>
    page.getByTestId(`room-default-role-option-${roleKey}`),

  uploadFileTypeModeSelect: (page: Page): Locator =>
    page.getByTestId("upload-file-type-mode"),

  uploadFileTypeOption: (page: Page, mode: string): Locator =>
    page.getByRole("option", { name: tRoom(`config.uploadFileType.mode.${mode}`) }),

  uploadFileTypeExtensionsInput: (page: Page): Locator =>
    page.locator("aside").first().locator("#upload-file-type-extensions"),

  saveRoomConfigButton: (page: Page): Locator =>
    page.locator("aside").first().getByRole("button", {
      name: tRoom("config.save.saveConfig"),
    }),

  capacityInfo: (page: Page): Locator =>
    page.locator("aside").first().getByText(tRoom("capacity.title")).locator(".."),

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

  fileNames: (page: Page): Locator =>
    page.locator("div.group.relative.flex.items-center.gap-3.rounded-lg.border p.text-sm.font-medium"),

  fileSelectAllButton: (page: Page): Locator =>
    page.locator("aside").last().getByRole("button", { name: tRoom("fileManager.selectAll") }),

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

  filePreviewCloseButton: (page: Page): Locator =>
    RoomScreen.filePreviewDialog(page).locator('button[title="Close"]'),

  // Download policy & access code protection
  filePolicySettingsButton: (page: Page, fileName: string): Locator =>
    RoomScreen.fileCards(page)
      .filter({ has: page.getByText(fileName, { exact: true }) })
      .first()
      .locator(`button[title='${tRoom("downloadPolicy.settingsTitle")}']`),

  fileProtectedBadge: (page: Page, fileName: string): Locator =>
    RoomScreen.fileCards(page)
      .filter({ has: page.getByText(fileName, { exact: true }) })
      .first()
      .getByText(tRoom("downloadPolicy.protectedBadge")),

  downloadPolicyDialog: (page: Page): Locator =>
    page.getByRole("dialog").filter({ hasText: tRoom("downloadPolicy.title") }),

  downloadPolicyModeSelect: (page: Page): Locator =>
    RoomScreen.downloadPolicyDialog(page).getByRole("combobox").first(),

  downloadPolicySaveButton: (page: Page): Locator =>
    RoomScreen.downloadPolicyDialog(page).getByRole("button", {
      name: tRoom("downloadPolicy.save"),
    }),

  reusableCodeInput: (page: Page): Locator =>
    RoomScreen.downloadPolicyDialog(page).getByPlaceholder(
      tRoom("downloadPolicy.reusableCodePlaceholder"),
    ),

  oneTimeCodesTextarea: (page: Page): Locator =>
    RoomScreen.downloadPolicyDialog(page).getByPlaceholder(
      tRoom("downloadPolicy.oneTimeCodesPlaceholder"),
    ),

  redeemDialog: (page: Page): Locator =>
    page.getByRole("dialog").filter({ hasText: tRoom("downloadPolicy.redeemTitle") }),

  redeemCodeInput: (page: Page): Locator =>
    RoomScreen.redeemDialog(page).getByPlaceholder(
      tRoom("downloadPolicy.accessCodePlaceholder"),
    ),

  redeemSubmitButton: (page: Page): Locator =>
    RoomScreen.redeemDialog(page).getByRole("button", {
      name: tRoom("downloadPolicy.redeemSubmit"),
    }),

  redeemError: (page: Page): Locator =>
    RoomScreen.redeemDialog(page).locator("p.text-destructive"),

  identityCodeDisclosure: (page: Page): Locator =>
    page.getByTestId("identity-code-disclosure"),

  disclosedIdentityCode: (page: Page): Locator =>
    page.getByTestId("disclosed-identity-code"),

  enterRoomAfterDisclosure: (page: Page): Locator =>
    page.getByTestId("enter-room"),

  identityRedeemOpenButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("identity.redeemAction") }),

  identityRedeemDialog: (page: Page): Locator =>
    page.getByRole("dialog").filter({ hasText: tRoom("identity.redeemTitle") }),

  identityRedeemInput: (page: Page): Locator =>
    RoomScreen.identityRedeemDialog(page).getByTestId("identity-redeem-input"),

  identityRedeemSubmitButton: (page: Page): Locator =>
    RoomScreen.identityRedeemDialog(page).getByRole("button", {
      name: tRoom("identity.redeemConfirm"),
    }),

  membersButton: (page: Page): Locator =>
    page.getByRole("button", { name: tRoom("identity.managePermissions") }),

  editorContainer: (page: Page): Locator =>
    page.locator(".tiptap-editor-container").first(),

  toast: (page: Page): Locator =>
    page.locator(
      "[data-state='open'][data-swipe-direction], [data-state='open'][data-sonner-toast], [data-state='open'][role='status'], [data-state='open'][role='alert']",
    ).first(),

  closeRoomPasswordError: (page: Page): Locator =>
    page.getByRole("dialog").locator("p.text-destructive, [class*='destructive'] p").first(),

  settingDesktopNotifications: (page: Page): Locator =>
    page.getByTestId("setting-desktop-notifications"),

  settingDesktopNotificationType: (
    page: Page,
    kind: "message" | "file" | "link" | "room",
    action:
      | "created"
      | "updated"
      | "deleted"
      | "address_changed"
      | "roles_changed"
      | "settings_changed",
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
};
