// Mock API service for room and chat operations
import type { Message, RoomDetails, RoomSettings } from "@/lib/types";

// Mock: Get room details
export const getRoomDetails = async (roomId: string): Promise<RoomDetails> => {
  // Simulate API delay
  await new Promise((resolve) => setTimeout(resolve, 300));

  return {
    id: roomId,
    currentSize: 15.5,
    maxSize: 100,
    settings: {
      expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
      passwordProtected: true,
      password: "demo123", // pragma: allowlist secret
      maxViews: 100,
    },
    permissions: ["read", "edit", "share", "delete"],
  };
};

// Mock: Update room settings
export const updateRoomSettings = async (
  roomId: string,
  settings: Partial<RoomSettings>,
): Promise<void> => {
  console.log("[API Mock] updateRoomSettings:", { roomId, settings });
  await new Promise((resolve) => setTimeout(resolve, 300));
};

// Mock: Get messages
export const getMessages = async (roomId: string): Promise<Message[]> => {
  await new Promise((resolve) => setTimeout(resolve, 300));

  return [
    {
      id: "msg_1",
      content: "欢迎来到 Elizabeth 房间！这是一个安全的文件分享空间。",
      timestamp: new Date(Date.now() - 3600000).toISOString(),
      user: "guest_1",
    },
    {
      id: "msg_2",
      content:
        "你可以使用 **Markdown** 格式来编写消息，支持 `代码` 和其他格式。",
      timestamp: new Date(Date.now() - 1800000).toISOString(),
      user: "guest_2",
    },
    {
      id: "msg_3",
      content:
        "# 标题示例\n\n- 列表项 1\n- 列表项 2\n\n这是一个功能完整的协作空间！",
      timestamp: new Date(Date.now() - 900000).toISOString(),
      user: "guest_1",
    },
  ];
};

// Mock: Post new message
export const postMessage = async (
  roomId: string,
  content: string,
): Promise<Message> => {
  console.log("[API Mock] postMessage:", { roomId, content });
  await new Promise((resolve) => setTimeout(resolve, 300));

  return {
    id: `msg_${Date.now()}`,
    content,
    timestamp: new Date().toISOString(),
    user: "guest_current",
  };
};

// Mock: Update message
export const updateMessage = async (
  messageId: string,
  content: string,
): Promise<Message> => {
  console.log("[API Mock] updateMessage:", { messageId, content });
  await new Promise((resolve) => setTimeout(resolve, 300));

  return {
    id: messageId,
    content,
    timestamp: new Date().toISOString(),
    user: "guest_edited",
  };
};

// Mock: Delete message
export const deleteMessage = async (messageId: string): Promise<void> => {
  console.log("[API Mock] deleteMessage:", messageId);
  await new Promise((resolve) => setTimeout(resolve, 300));
};
