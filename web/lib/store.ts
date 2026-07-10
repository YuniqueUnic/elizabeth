// Global state management using Zustand
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { LocalMessage, Message, Theme, TokenInfo } from "./types";
import {
  createDefaultDesktopNotificationTypes,
  normalizeDesktopNotificationTypes,
  type DesktopNotificationAction,
  type DesktopNotificationKind,
  type DesktopNotificationPermission,
  type DesktopNotificationTypes,
} from "./desktop-notifications";
import { getRoomToken } from "./utils/api";
import { hasValidToken } from "../api/authService";
import {
  deleteMessage,
  getMessagePage,
  postMessage,
  updateMessage,
} from "@/api/messageService";
import {
  type MessageContentEvent,
  mergeMessagePage,
  messageFromContentEvent,
  rebasePendingMessages,
  removeMessage,
  replacePendingMessage,
  replaceSavedMessage,
} from "./messages/message-cache";

export type MessageLoadStatus = "idle" | "loading" | "ready" | "error";

type MessageSaveResult =
  | { kind: "remove"; id: string }
  | { kind: "replace"; pendingId: string; message: Message }
  | { kind: "upsert"; message: Message };

interface AppState {
  // Locale
  locale: "zh" | "en";
  setLocale: (locale: "zh" | "en") => void;

  // Theme management
  theme: Theme;
  setTheme: (theme: Theme) => void;
  cycleTheme: () => void;

  // Settings
  sendOnEnter: boolean;
  setSendOnEnter: (value: boolean) => void;

  // Auto-scroll
  autoScroll: boolean;
  setAutoScroll: (value: boolean) => void;

  // Browser desktop notifications
  desktopNotificationsEnabled: boolean;
  setDesktopNotificationsEnabled: (value: boolean) => void;
  desktopNotificationShowContent: boolean;
  setDesktopNotificationShowContent: (value: boolean) => void;
  desktopNotificationPermission: DesktopNotificationPermission;
  setDesktopNotificationPermission: (value: DesktopNotificationPermission) => void;
  desktopNotificationTypes: DesktopNotificationTypes;
  setDesktopNotificationType: (
    kind: DesktopNotificationKind,
    action: DesktopNotificationAction,
    value: boolean,
  ) => void;

  // Editor and message font sizes
  editorFontSize: number;
  setEditorFontSize: (size: number) => void;
  toolbarButtonSize: number;
  setToolbarButtonSize: (size: number) => void;
  messageFontSize: number;
  setMessageFontSize: (size: number) => void;

  // File selection
  selectedFiles: Set<string>;
  toggleFileSelection: (fileId: string) => void;
  clearFileSelection: () => void;

  selectedMessages: Set<string>;
  toggleMessageSelection: (messageId: string) => void;
  clearMessageSelection: () => void;
  selectAllMessages: (messageIds: string[]) => void;
  invertMessageSelection: (messageIds: string[]) => void;

  // Metadata settings for copy and download (separate configurations)
  includeMetadataInCopy: boolean;
  setIncludeMetadataInCopy: (value: boolean) => void;
  includeMetadataInDownload: boolean;
  setIncludeMetadataInDownload: (value: boolean) => void;
  // Legacy: keep for backward compatibility, but prefer using copy/download specific settings
  includeMetadataInExport: boolean;
  setIncludeMetadataInExport: (value: boolean) => void;

  selectAllFiles: (fileIds: string[]) => void;
  invertFileSelection: (fileIds: string[]) => void;

  // Sidebar collapse states
  leftSidebarCollapsed: boolean;
  toggleLeftSidebar: () => void;

  // Current room
  currentRoomId: string;
  setCurrentRoomId: (roomId: string) => void;

  // File preview (from message link clicks)
  previewFileId: string | null;
  setPreviewFileId: (id: string | null) => void;

  // Authentication state (derived from localStorage tokens)
  isAuthenticated: (roomName?: string) => boolean;
  getCurrentRoomToken: () => TokenInfo | null;

  // heti
  useHeti: boolean;
  setUseHeti: (value: boolean) => void;

  // UI Preferences
  showDeleteConfirmation: boolean;
  setShowDeleteConfirmation: (show: boolean) => void;

  // Local message management for explicit saving
  messages: LocalMessage[];
  messageCacheRoomId: string | null;
  messageCacheGeneration: number;
  messageInitialStatus: MessageLoadStatus;
  messageOlderStatus: MessageLoadStatus;
  messageNextCursor: string | null;
  messageHasMore: boolean;
  messageNextSequenceNumber: number;
  ensureMessagesLoaded: (roomId: string) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  refreshLatestMessages: () => Promise<void>;
  applyMessageCreated: (payload: MessageContentEvent) => void;
  applyMessageUpdated: (payload: MessageContentEvent) => void;
  applyMessageDeleted: (messageId: string) => void;
  addMessage: (content: string) => void;
  updateMessageContent: (messageId: string, content: string) => void;
  markMessageForDeletion: (messageId: string) => void;
  revertMessageChanges: (messageId: string) => void;
  hasUnsavedChanges: () => boolean;
  isSaving: boolean;
  saveMessages: () => Promise<void>;

  // Composer state (draft + edit mode). Not persisted.
  composerContent: string;
  setComposerContent: (content: string) => void;
  composerEditingMessageId: string | null;
  beginEditMessage: (messageId: string) => void;
  cancelEditMessage: () => void;

  // Request the active editor to insert markdown at the cursor position.
  composerInsertRequest: { id: number; markdown: string } | null;
  requestInsertMarkdown: (markdown: string) => void;
  clearInsertMarkdownRequest: (requestId: number) => void;

  // Transfer state (uploads + downloads)
  transfers: Record<string, import("./transfer-types").TransferItem>;
  addTransfer: (item: import("./transfer-types").TransferItem) => void;
  updateTransferProgress: (id: string, progress: import("./transfer-types").TransferProgress) => void;
  updateTransferStatus: (id: string, status: import("./transfer-types").TransferStatus, error?: string) => void;
  removeTransfer: (id: string) => void;
  cancelTransfer: (id: string) => void;

  // Global redirect state (for room renaming)
  roomRedirectTarget: string | null;
  setRoomRedirectTarget: (target: string | null) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Locale
      locale: "zh",
      setLocale: (locale) => {
        set({ locale });
        if (typeof window !== "undefined") {
          document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
        }
      },

      // Theme
      theme: "system",
      setTheme: (theme) => set({ theme }),
      cycleTheme: () => {
        const current = get().theme;
        const next = current === "dark"
          ? "light"
          : current === "light"
          ? "system"
          : "dark";
        set({ theme: next });
      },

      // Settings
      sendOnEnter: true,
      setSendOnEnter: (value) => set({ sendOnEnter: value }),

      // Auto-scroll
      autoScroll: true,
      setAutoScroll: (value) => set({ autoScroll: value }),

      // Browser desktop notifications
      desktopNotificationsEnabled: false,
      setDesktopNotificationsEnabled: (value) =>
        set({ desktopNotificationsEnabled: value }),
      desktopNotificationShowContent: true,
      setDesktopNotificationShowContent: (value) =>
        set({ desktopNotificationShowContent: value }),
      desktopNotificationPermission: "default",
      setDesktopNotificationPermission: (value) =>
        set({ desktopNotificationPermission: value }),
      desktopNotificationTypes: createDefaultDesktopNotificationTypes(),
      setDesktopNotificationType: (kind, action, value) =>
        set((state) => ({
          desktopNotificationTypes: {
            ...state.desktopNotificationTypes,
            [kind]: {
              ...state.desktopNotificationTypes[kind],
              [action]: value,
            },
          },
        })),

      // Editor and message font sizes
      editorFontSize: 15,
      setEditorFontSize: (size) => set({ editorFontSize: size }),
      toolbarButtonSize: 28,
      setToolbarButtonSize: (size) => set({ toolbarButtonSize: size }),
      messageFontSize: 14,
      setMessageFontSize: (size) => set({ messageFontSize: size }),

      // File selection
      selectedFiles: new Set(),
      toggleFileSelection: (fileId) => {
        const selected = new Set(get().selectedFiles);
        if (selected.has(fileId)) {
          selected.delete(fileId);
        } else {
          selected.add(fileId);
        }
        set({ selectedFiles: selected });
      },
      clearFileSelection: () => set({ selectedFiles: new Set() }),

      selectedMessages: new Set(),
      toggleMessageSelection: (messageId) => {
        const selected = new Set(get().selectedMessages);
        if (selected.has(messageId)) {
          selected.delete(messageId);
        } else {
          selected.add(messageId);
        }
        set({ selectedMessages: selected });
      },
      clearMessageSelection: () => set({ selectedMessages: new Set() }),
      selectAllMessages: (messageIds) =>
        set({ selectedMessages: new Set(messageIds) }),
      invertMessageSelection: (messageIds) => {
        const selected = new Set(get().selectedMessages);
        const newSelected = new Set<string>();
        messageIds.forEach((id) => {
          if (!selected.has(id)) {
            newSelected.add(id);
          }
        });
        set({ selectedMessages: newSelected });
      },

      includeMetadataInCopy: false,
      setIncludeMetadataInCopy: (value) =>
        set({ includeMetadataInCopy: value }),
      includeMetadataInDownload: true,
      setIncludeMetadataInDownload: (value) =>
        set({ includeMetadataInDownload: value }),
      includeMetadataInExport: true,
      setIncludeMetadataInExport: (value) =>
        set({ includeMetadataInExport: value }),

      selectAllFiles: (fileIds) => set({ selectedFiles: new Set(fileIds) }),
      invertFileSelection: (fileIds) => {
        const selected = new Set(get().selectedFiles);
        const newSelected = new Set<string>();
        fileIds.forEach((id) => {
          if (!selected.has(id)) {
            newSelected.add(id);
          }
        });
        set({ selectedFiles: newSelected });
      },

      // Sidebar
      leftSidebarCollapsed: false,
      toggleLeftSidebar: () =>
        set({ leftSidebarCollapsed: !get().leftSidebarCollapsed }),

      // Room
      currentRoomId: "demo-room-123",
      setCurrentRoomId: (roomId) =>
        set((state) => {
          if (state.currentRoomId === roomId) return {};
          return {
            currentRoomId: roomId,
            messages: [],
            messageCacheRoomId: roomId,
            messageCacheGeneration: state.messageCacheGeneration + 1,
            messageInitialStatus: "idle",
            messageOlderStatus: "idle",
            messageNextCursor: null,
            messageHasMore: false,
            messageNextSequenceNumber: 0,
            selectedMessages: new Set(),
            selectedFiles: new Set(),
            composerContent: "",
            composerEditingMessageId: null,
            composerInsertRequest: null,
            previewFileId: null,
          };
        }),

      // File preview
      previewFileId: null,
      setPreviewFileId: (id) => set({ previewFileId: id }),

      // Authentication (derived from localStorage tokens)
      isAuthenticated: (roomName) => {
        const room = roomName || get().currentRoomId;
        return hasValidToken(room);
      },
      getCurrentRoomToken: () => {
        const roomId = get().currentRoomId;
        return getRoomToken(roomId);
      },

      // heti
      useHeti: false,
      setUseHeti: (value) => set({ useHeti: value }),

      // UI Preferences
      showDeleteConfirmation: true,
      setShowDeleteConfirmation: (show) =>
        set({ showDeleteConfirmation: show }),

      // Local message management
      messages: [],
      messageCacheRoomId: null,
      messageCacheGeneration: 0,
      messageInitialStatus: "idle",
      messageOlderStatus: "idle",
      messageNextCursor: null,
      messageHasMore: false,
      messageNextSequenceNumber: 0,
      ensureMessagesLoaded: async (roomId) => {
        const state = get();
        if (!roomId) return;
        if (
          state.messageCacheRoomId === roomId &&
          (state.messageInitialStatus === "loading" ||
            state.messageInitialStatus === "ready")
        ) return;

        let generation = state.messageCacheGeneration;
        if (state.messageCacheRoomId !== roomId) {
          generation += 1;
          set({
            messages: [],
            messageCacheRoomId: roomId,
            messageCacheGeneration: generation,
            messageNextCursor: null,
            messageHasMore: false,
            messageNextSequenceNumber: 0,
            selectedMessages: new Set(),
          });
        }
        set({ messageInitialStatus: "loading" });

        try {
          const page = await getMessagePage(roomId);
          const latest = get();
          if (
            latest.messageCacheRoomId !== roomId ||
            latest.messageCacheGeneration !== generation
          ) return;

          set((current) => {
            const rebased = rebasePendingMessages(
              mergeMessagePage(current.messages, page.items),
              page.nextSequenceNumber,
            );
            return {
              messages: rebased.messages,
              messageInitialStatus: "ready",
              messageOlderStatus: "idle",
              messageNextCursor: page.nextCursor,
              messageHasMore: page.hasMore,
              messageNextSequenceNumber: rebased.nextSequenceNumber,
            };
          });
        } catch (error) {
          const latest = get();
          if (
            latest.messageCacheRoomId !== roomId ||
            latest.messageCacheGeneration !== generation
          ) return;
          set({ messageInitialStatus: "error" });
          throw error;
        }
      },
      loadOlderMessages: async () => {
        const state = get();
        const roomId = state.currentRoomId;
        if (
          !roomId || state.messageInitialStatus !== "ready" ||
          state.messageOlderStatus === "loading" || !state.messageHasMore ||
          !state.messageNextCursor
        ) return;

        const generation = state.messageCacheGeneration;
        const cursor = state.messageNextCursor;
        set({ messageOlderStatus: "loading" });
        try {
          const page = await getMessagePage(roomId, cursor);
          const latest = get();
          if (
            latest.messageCacheRoomId !== roomId ||
            latest.messageCacheGeneration !== generation
          ) return;

          set((current) => ({
            messages: mergeMessagePage(current.messages, page.items),
            messageOlderStatus: "ready",
            messageNextCursor: page.nextCursor,
            messageHasMore: page.hasMore,
          }));
        } catch (error) {
          const latest = get();
          if (
            latest.messageCacheRoomId !== roomId ||
            latest.messageCacheGeneration !== generation
          ) return;
          set({ messageOlderStatus: "error" });
          throw error;
        }
      },
      refreshLatestMessages: async () => {
        const state = get();
        const roomId = state.currentRoomId;
        if (!roomId || state.messageInitialStatus !== "ready") return;
        const generation = state.messageCacheGeneration;
        const page = await getMessagePage(roomId);
        const latest = get();
        if (
          latest.messageCacheRoomId !== roomId ||
          latest.messageCacheGeneration !== generation
        ) return;

        set((current) => {
          const rebased = rebasePendingMessages(
            mergeMessagePage(current.messages, page.items),
            page.nextSequenceNumber,
          );
          return {
            messages: rebased.messages,
            messageNextSequenceNumber: rebased.nextSequenceNumber,
          };
        });
      },
      applyMessageCreated: (payload) =>
        set((state) => {
          if (state.messageCacheRoomId !== payload.room_name) return {};
          const existing = payload.content_id == null
            ? undefined
            : state.messages.find((message) =>
              message.id === String(payload.content_id)
            );
          const message = messageFromContentEvent(payload, existing);
          if (!message) return {};
          const rebased = rebasePendingMessages(
            mergeMessagePage(state.messages, [message]),
            Math.max(
              state.messageNextSequenceNumber,
              (message.sequence_number ?? 0) + 1,
            ),
          );
          return {
            messages: rebased.messages,
            messageNextSequenceNumber: rebased.nextSequenceNumber,
          };
        }),
      applyMessageUpdated: (payload) =>
        set((state) => {
          if (state.messageCacheRoomId !== payload.room_name) return {};
          const existing = payload.content_id == null
            ? undefined
            : state.messages.find((message) =>
              message.id === String(payload.content_id)
            );
          const message = messageFromContentEvent(payload, existing);
          if (!message) return {};
          return { messages: mergeMessagePage(state.messages, [message]) };
        }),
      applyMessageDeleted: (messageId) =>
        set((state) => {
          const selectedMessages = new Set(state.selectedMessages);
          selectedMessages.delete(messageId);
          const wasEditing = state.composerEditingMessageId === messageId;
          return {
            messages: removeMessage(state.messages, messageId),
            selectedMessages,
            composerEditingMessageId: wasEditing
              ? null
              : state.composerEditingMessageId,
            composerContent: wasEditing ? "" : state.composerContent,
          };
        }),
      addMessage: (content) => {
        set((state) => {
          const sequenceNumber = state.messageNextSequenceNumber;
          const newMessage: LocalMessage = {
            id: `temp-${Date.now()}-${sequenceNumber}`,
            content,
            timestamp: new Date().toISOString(),
            isOwn: true,
            isNew: true,
            sequence_number: sequenceNumber,
          };
          return {
            messages: [...state.messages, newMessage],
            messageNextSequenceNumber: sequenceNumber + 1,
          };
        });
      },
      updateMessageContent: (messageId, content) => {
        set((state) => ({
          messages: state.messages.map((m) => {
            if (m.id === messageId) {
              // Store original content on first edit, if not already stored
              const originalContent = m.originalContent ?? m.content;
              return {
                ...m,
                content,
                isDirty: !m.isNew, // Don't mark new messages as dirty
                originalContent,
              };
            }
            return m;
          }),
        }));
      },
      markMessageForDeletion: (messageId) => {
        set((state) => ({
          messages: state.messages.map((m) =>
            m.id === messageId ? { ...m, isPendingDelete: true } : m
          ),
        }));
      },
      revertMessageChanges: (messageId: string) => {
        set((state) => ({
          messages: state.messages.map((m) => {
            if (m.id === messageId) {
              const revertedMessage = { ...m };
              // Revert pending deletion
              if (revertedMessage.isPendingDelete) {
                revertedMessage.isPendingDelete = false;
              }
              // Revert content edit
              if (revertedMessage.isDirty && revertedMessage.originalContent) {
                revertedMessage.content = revertedMessage.originalContent;
                revertedMessage.isDirty = false;
                revertedMessage.originalContent = undefined;
              }
              return revertedMessage;
            }
            return m;
          }),
        }));
      },
      hasUnsavedChanges: () => {
        const messages = get().messages;
        const hasActiveUploads = Object.values(get().transfers).some(
          (t) => t.status === "active" && t.direction === "upload",
        );
        return (
          messages.some((m) => m.isNew || m.isDirty || m.isPendingDelete) ||
          hasActiveUploads
        );
      },
      isSaving: false,
      saveMessages: async () => {
        if (get().isSaving) return;
        set({ isSaving: true });
        try {
          const {
            messages,
            currentRoomId,
            messageCacheGeneration,
          } = get();
          const unsavedMessages = messages.filter(
            (m) => m.isNew || m.isDirty || m.isPendingDelete,
          );

          if (unsavedMessages.length === 0) {
            return;
          }

          const results = await Promise.allSettled(
            unsavedMessages.map(async (msg): Promise<MessageSaveResult> => {
              if (msg.isPendingDelete) {
                if (!msg.isNew) {
                  await deleteMessage(currentRoomId, msg.id);
                }
                return { kind: "remove", id: msg.id };
              } else if (msg.isNew) {
                const saved = await postMessage(
                  currentRoomId,
                  msg.content,
                  msg.sequence_number,
                );
                return { kind: "replace", pendingId: msg.id, message: saved };
              } else if (msg.isDirty) {
                const updated = await updateMessage(currentRoomId, msg.id, msg.content);
                return { kind: "upsert", message: updated };
              }
              return { kind: "upsert", message: msg };
            })
          );

          if (
            get().currentRoomId === currentRoomId &&
            get().messageCacheGeneration === messageCacheGeneration
          ) {
            set((state) => {
              let nextMessages = state.messages;
              for (const result of results) {
                if (result.status !== "fulfilled") continue;
                const saved = result.value;
                if (saved.kind === "remove") {
                  nextMessages = removeMessage(nextMessages, saved.id);
                } else if (saved.kind === "replace") {
                  nextMessages = replacePendingMessage(
                    nextMessages,
                    saved.pendingId,
                    saved.message,
                  );
                } else {
                  nextMessages = replaceSavedMessage(nextMessages, saved.message);
                }
              }
              const rebased = rebasePendingMessages(
                nextMessages,
                state.messageNextSequenceNumber,
              );
              return {
                messages: rebased.messages,
                messageNextSequenceNumber: rebased.nextSequenceNumber,
              };
            });
          }

          const failed = results.find((result) => result.status === "rejected");
          if (failed?.status === "rejected") throw failed.reason;
        } finally {
          set({ isSaving: false });
        }
      },

      composerContent: "",
      setComposerContent: (content) => set({ composerContent: content }),
      composerEditingMessageId: null,
      beginEditMessage: (messageId) => {
        const message = get().messages.find((m) => m.id === messageId);
        set({
          composerEditingMessageId: messageId,
          composerContent: message?.content ?? "",
        });
      },
      cancelEditMessage: () =>
        set({ composerEditingMessageId: null, composerContent: "" }),

      composerInsertRequest: null,
      requestInsertMarkdown: (markdown) =>
        set({ composerInsertRequest: { id: Date.now(), markdown } }),
      clearInsertMarkdownRequest: (requestId) =>
        set((state) =>
          state.composerInsertRequest?.id === requestId
            ? { composerInsertRequest: null }
            : {}
        ),

      // Transfer state (uploads + downloads)
      transfers: {},
      addTransfer: (item) =>
        set((state) => ({ transfers: { ...state.transfers, [item.id]: item } })),
      updateTransferProgress: (id, progress) =>
        set((state) => {
          const existing = state.transfers[id];
          if (!existing) return {};
          return { transfers: { ...state.transfers, [id]: { ...existing, progress } } };
        }),
      updateTransferStatus: (id, status, error) =>
        set((state) => {
          const existing = state.transfers[id];
          if (!existing) return {};
          return { transfers: { ...state.transfers, [id]: { ...existing, status, error } } };
        }),
      removeTransfer: (id) =>
        set((state) => {
          const { [id]: _, ...rest } = state.transfers;
          return { transfers: rest };
        }),
      cancelTransfer: (id) => {
        const transfer = get().transfers[id];
        if (transfer) {
          transfer.abortController.abort();
          set((state) => ({
            transfers: {
              ...state.transfers,
              [id]: { ...transfer, status: "cancelled" as const },
            },
          }));
        }
      },

      // Global redirect state
      roomRedirectTarget: null,
      setRoomRedirectTarget: (target) => set({ roomRedirectTarget: target }),
    }),
    {
      name: "elizabeth-storage",
      storage: createJSONStorage(() => localStorage),
      // Only persist a subset of the state
      partialize: (state) => ({
        // Persist locale and UI preferences
        locale: state.locale,
        sendOnEnter: state.sendOnEnter,
        autoScroll: state.autoScroll,
        includeMetadataInCopy: state.includeMetadataInCopy,
        includeMetadataInDownload: state.includeMetadataInDownload,
        includeMetadataInExport: state.includeMetadataInExport,
        editorFontSize: state.editorFontSize,
        toolbarButtonSize: state.toolbarButtonSize,
        messageFontSize: state.messageFontSize,
        useHeti: state.useHeti,
        showDeleteConfirmation: state.showDeleteConfirmation,
        desktopNotificationsEnabled: state.desktopNotificationsEnabled,
        desktopNotificationShowContent: state.desktopNotificationShowContent,
        desktopNotificationTypes: state.desktopNotificationTypes,
      }),
      merge: (persisted, current) => {
        const state = persisted as Partial<AppState> | undefined;
        return {
          ...current,
          ...state,
          desktopNotificationShowContent:
            state?.desktopNotificationShowContent ?? true,
          desktopNotificationPermission: "default",
          desktopNotificationTypes: normalizeDesktopNotificationTypes(
            state?.desktopNotificationTypes,
          ),
        };
      },
    },
  ),
);

if (typeof window !== "undefined" && process.env.NODE_ENV !== "production") {
  (window as any).__ELIZABETH_STORE__ = useAppStore;
}
