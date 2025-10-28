// Global state management using Zustand
import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Theme, TokenInfo } from "./types";
import { getRoomToken } from "./utils/api";
import { hasValidToken } from "../api/authService";

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
      setCurrentRoomId: (roomId) => set({ currentRoomId: roomId }),

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
    }),
    {
      name: "elizabeth-storage",
      partialize: (state) => ({
        theme: state.theme,
        sendOnEnter: state.sendOnEnter,
        includeMetadataInExport: state.includeMetadataInExport,
        editorFontSize: state.editorFontSize,
        toolbarButtonSize: state.toolbarButtonSize,
        messageFontSize: state.messageFontSize,
        useHeti: state.useHeti,
      }),
    },
  ),
);
