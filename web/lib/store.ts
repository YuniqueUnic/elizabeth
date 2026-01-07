// Global state management using Zustand
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { LocalMessage, Message, Theme, TokenInfo } from "./types";
import { getRoomToken } from "./utils/api";
import { hasValidToken } from "../api/authService";
import {
  deleteMessage,
  getMessages,
  postMessage,
  updateMessage,
} from "@/api/messageService";

interface AppState {
  // Theme management
  theme: Theme;
  setTheme: (theme: Theme) => void;
  cycleTheme: () => void;

  // Settings
  sendOnEnter: boolean;
  setSendOnEnter: (value: boolean) => void;

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
  setMessages: (messages: Message[]) => void;
  addMessage: (content: string) => void;
  updateMessageContent: (messageId: string, content: string) => void;
  markMessageForDeletion: (messageId: string) => void;
  revertMessageChanges: (messageId: string) => void;
  hasUnsavedChanges: () => boolean;
  saveMessages: () => Promise<void>;
  syncMessagesFromServer: () => Promise<void>;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
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
        set({ currentRoomId: roomId, messages: [] }),

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
      setMessages: (messages) =>
        set({
          messages: messages.map((m) => ({
            ...m,
            isNew: false,
            isDirty: false,
            isPendingDelete: false,
          })),
        }),
      addMessage: (content) => {
        const newMessage: LocalMessage = {
          id: `temp-${Date.now()}`,
          content,
          timestamp: new Date().toISOString(),
          isOwn: true,
          isNew: true,
        };
        set((state) => ({ messages: [...state.messages, newMessage] }));
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
        return messages.some((m) => m.isNew || m.isDirty || m.isPendingDelete);
      },
      saveMessages: async () => {
        const { messages, currentRoomId } = get();
        const unsavedMessages = messages.filter(
          (m) => m.isNew || m.isDirty || m.isPendingDelete,
        );

        if (unsavedMessages.length === 0) {
          return;
        }

        const promises = unsavedMessages.map((msg) => {
          if (msg.isPendingDelete) {
            // For new messages that are deleted before saving, just remove them locally
            if (msg.isNew) {
              return Promise.resolve();
            }
            return deleteMessage(currentRoomId, msg.id);
          }
          if (msg.isNew) {
            return postMessage(currentRoomId, msg.content);
          }
          if (msg.isDirty) {
            return updateMessage(currentRoomId, msg.id, msg.content);
          }
          return Promise.resolve();
        });

        await Promise.all(promises);

        // Refetch messages from the server to ensure consistency
        const updatedMessages = await getMessages(currentRoomId);
        set({
          messages: updatedMessages.map((m) => ({
            ...m,
            isNew: false,
            isDirty: false,
            isPendingDelete: false,
          })),
        });
      },

      syncMessagesFromServer: async () => {
        const { currentRoomId } = get();
        if (!currentRoomId) return;

        const serverMessages = await getMessages(currentRoomId);

        set((state) => {
          const pending = state.messages.filter((m) =>
            m.isNew || m.isDirty || m.isPendingDelete
          );
          const pendingById = new Map(pending.map((m) => [m.id, m]));
          const serverIds = new Set(serverMessages.map((m) => m.id));

          const merged = serverMessages.map((msg) => {
            const existing = pendingById.get(msg.id);
            if (existing) return existing;
            return {
              ...msg,
              isNew: false,
              isDirty: false,
              isPendingDelete: false,
              originalContent: undefined,
            } satisfies LocalMessage;
          });

          // Keep local pending messages that don't exist on server yet (e.g., temp ids).
          for (const msg of pending) {
            if (!serverIds.has(msg.id)) {
              merged.push(msg);
            }
          }

          merged.sort((a, b) =>
            new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
          );

          return { messages: merged };
        });
      },
    }),
    {
      name: "elizabeth-storage",
      storage: createJSONStorage(() => localStorage),
      // Only persist a subset of the state
      partialize: (state) => ({
        // Persist UI preferences
        sendOnEnter: state.sendOnEnter,
        includeMetadataInCopy: state.includeMetadataInCopy,
        includeMetadataInDownload: state.includeMetadataInDownload,
        includeMetadataInExport: state.includeMetadataInExport,
        editorFontSize: state.editorFontSize,
        toolbarButtonSize: state.toolbarButtonSize,
        messageFontSize: state.messageFontSize,
        useHeti: state.useHeti,
        showDeleteConfirmation: state.showDeleteConfirmation,
      }),
    },
  ),
);
