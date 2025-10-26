// Global state management using Zustand
import { create } from "zustand"
import { persist } from "zustand/middleware"
import type { Theme } from "./types"

interface AppState {
  // Theme management
  theme: Theme
  setTheme: (theme: Theme) => void
  cycleTheme: () => void

  // Settings
  sendOnEnter: boolean
  setSendOnEnter: (value: boolean) => void

  // File selection
  selectedFiles: Set<string>
  toggleFileSelection: (fileId: string) => void
  clearFileSelection: () => void

  selectedMessages: Set<string>
  toggleMessageSelection: (messageId: string) => void
  clearMessageSelection: () => void
  selectAllMessages: (messageIds: string[]) => void
  invertMessageSelection: (messageIds: string[]) => void

  includeMetadataInExport: boolean
  setIncludeMetadataInExport: (value: boolean) => void

  selectAllFiles: (fileIds: string[]) => void
  invertFileSelection: (fileIds: string[]) => void

  // Sidebar collapse states
  leftSidebarCollapsed: boolean
  toggleLeftSidebar: () => void

  // Current room
  currentRoomId: string
  setCurrentRoomId: (roomId: string) => void
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Theme
      theme: "system",
      setTheme: (theme) => set({ theme }),
      cycleTheme: () => {
        const current = get().theme
        const next = current === "dark" ? "light" : current === "light" ? "system" : "dark"
        set({ theme: next })
      },

      // Settings
      sendOnEnter: true,
      setSendOnEnter: (value) => set({ sendOnEnter: value }),

      // File selection
      selectedFiles: new Set(),
      toggleFileSelection: (fileId) => {
        const selected = new Set(get().selectedFiles)
        if (selected.has(fileId)) {
          selected.delete(fileId)
        } else {
          selected.add(fileId)
        }
        set({ selectedFiles: selected })
      },
      clearFileSelection: () => set({ selectedFiles: new Set() }),

      selectedMessages: new Set(),
      toggleMessageSelection: (messageId) => {
        const selected = new Set(get().selectedMessages)
        if (selected.has(messageId)) {
          selected.delete(messageId)
        } else {
          selected.add(messageId)
        }
        set({ selectedMessages: selected })
      },
      clearMessageSelection: () => set({ selectedMessages: new Set() }),
      selectAllMessages: (messageIds) => set({ selectedMessages: new Set(messageIds) }),
      invertMessageSelection: (messageIds) => {
        const selected = new Set(get().selectedMessages)
        const newSelected = new Set<string>()
        messageIds.forEach((id) => {
          if (!selected.has(id)) {
            newSelected.add(id)
          }
        })
        set({ selectedMessages: newSelected })
      },

      includeMetadataInExport: true,
      setIncludeMetadataInExport: (value) => set({ includeMetadataInExport: value }),

      selectAllFiles: (fileIds) => set({ selectedFiles: new Set(fileIds) }),
      invertFileSelection: (fileIds) => {
        const selected = new Set(get().selectedFiles)
        const newSelected = new Set<string>()
        fileIds.forEach((id) => {
          if (!selected.has(id)) {
            newSelected.add(id)
          }
        })
        set({ selectedFiles: newSelected })
      },

      // Sidebar
      leftSidebarCollapsed: false,
      toggleLeftSidebar: () => set({ leftSidebarCollapsed: !get().leftSidebarCollapsed }),

      // Room
      currentRoomId: "demo-room-123",
      setCurrentRoomId: (roomId) => set({ currentRoomId: roomId }),
    }),
    {
      name: "elizabeth-storage",
      partialize: (state) => ({
        theme: state.theme,
        sendOnEnter: state.sendOnEnter,
        includeMetadataInExport: state.includeMetadataInExport,
      }),
    },
  ),
)
