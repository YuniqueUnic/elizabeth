import { create } from "zustand";

interface AppState {
    currentRoomId: string;
    setCurrentRoomId: (id: string) => void;
    selectedMessages: string[];
    setSelectedMessages: (messages: string[]) => void;
    toggleMessageSelection: (messageId: string) => void;
    clearMessageSelection: () => void;
    selectedFiles: string[];
    setSelectedFiles: (files: string[]) => void;
    toggleFileSelection: (fileId: string) => void;
    clearFileSelection: () => void;
}

export const useAppStore = create<AppState>((set) => ({
    currentRoomId: "",
    setCurrentRoomId: (id) => set({ currentRoomId: id }),
    selectedMessages: [],
    setSelectedMessages: (messages) => set({ selectedMessages: messages }),
    toggleMessageSelection: (messageId) =>
        set((state) => ({
            selectedMessages: state.selectedMessages.includes(messageId)
                ? state.selectedMessages.filter((id) => id !== messageId)
                : [...state.selectedMessages, messageId],
        })),
    clearMessageSelection: () => set({ selectedMessages: [] }),
    selectedFiles: [],
    setSelectedFiles: (files) => set({ selectedFiles: files }),
    toggleFileSelection: (fileId) =>
        set((state) => ({
            selectedFiles: state.selectedFiles.includes(fileId)
                ? state.selectedFiles.filter((id) => id !== fileId)
                : [...state.selectedFiles, fileId],
        })),
    clearFileSelection: () => set({ selectedFiles: [] }),
}));
