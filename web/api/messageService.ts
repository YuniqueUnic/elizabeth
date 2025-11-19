/**
 * Message Service
 *
 * This service handles chat message operations using the Content API.
 * Messages are stored as RoomContent with content_type = ContentType.Text (0)
 */

import { API_BASE_URL, API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type {
  backendContentToMessage,
  BackendRoomContent,
  ContentType,
  Message,
} from "../lib/types";
import {
  backendContentToMessage as convertMessage,
  ContentType as CT,
  parseContentType,
} from "../lib/types";

// ============================================================================
// Message Request/Response Types
// ============================================================================

export interface PrepareUploadRequest {
  files: Array<{
    name: string;
    size: number;
    mime?: string;
    file_hash?: string;
  }>;
}

export interface PrepareUploadResponse {
  reservation_id: number;
  reserved_size: number;
  expires_at: string;
  current_size: number;
  remaining_size: number;
  max_size: number;
}

// ============================================================================
// Message Functions
// ============================================================================

/**
 * Get all messages for a room
 * Filters RoomContent for text-only content (content_type = 0 or text files)
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Array of messages
 */
export async function getMessages(
  roomName: string,
  token?: string,
): Promise<Message[]> {
  try {
    const authToken = token || await getValidToken(roomName);

    if (!authToken) {
      throw new Error("Authentication required to get messages");
    }

    const contents = await api.get<BackendRoomContent[]>(
      API_ENDPOINTS.content.base(roomName),
      undefined,
      { token: authToken },
    );

    const filteredContents = contents.filter((content) => {
      const contentType = parseContentType(content.content_type);
      return contentType === CT.Text ||
        (contentType === CT.File &&
          content.mime_type === "text/plain" &&
          content.file_name?.includes("message.txt"));
    });

    const messagesWithContent = await Promise.all(
      filteredContents.map(async (content) => {
        // If content text is missing and it's a text file, fetch it
        if (!content.text && content.mime_type === "text/plain") {
          try {
            const response = await fetch(
              `${API_BASE_URL}${
                API_ENDPOINTS.content.byId(roomName, String(content.id))
              }?token=${authToken}`,
              { headers: { "Accept": "text/plain" } },
            );
            if (response.ok) {
              const fileContent = await response.text();
              return { ...content, text: fileContent };
            }
            return { ...content, text: "[Failed to load content]" };
          } catch (error) {
            return { ...content, text: "[Failed to load content]" };
          }
        }
        return content;
      }),
    );

    return messagesWithContent
      .map(convertMessage)
      .sort((a, b) =>
        new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
      );
  } catch (error) {
    console.error("getMessages fallback to empty list:", error);
    return [];
  }
}

/**
 * Send a message to a room
 *
 * This uses a two-step process:
 * 1. Prepare upload to reserve space
 * 2. Upload the message as a text file using FormData
 *
 * @param roomName - The name of the room
 * @param content - The message content
 * @param token - Optional token for authentication
 * @returns The created message
 */
export async function postMessage(
  roomName: string,
  content: string,
  token?: string,
): Promise<Message> {
  try {
    const authToken = token || await getValidToken(roomName);

    if (!authToken) {
      throw new Error("Authentication required to send messages");
    }

    // Step 1: Prepare upload
    const textBytes = new TextEncoder().encode(content).length;
    const prepareRequest: PrepareUploadRequest = {
      files: [{
        name: "message.txt",
        size: textBytes,
        mime: "text/plain",
      }],
    };

    const prepareResponse = await api.post<PrepareUploadResponse>(
      API_ENDPOINTS.content.prepare(roomName),
      prepareRequest,
      { token: authToken },
    );

    // Step 2: Upload content as FormData (required by backend)
    const contentString = typeof content === "string"
      ? content
      : String(content);
    const formData = new FormData();
    const blob = new Blob([contentString], { type: "text/plain" });
    formData.append("file", blob, "message.txt");

    const response = await api.post<{ uploaded: BackendRoomContent[] }>(
      `${
        API_ENDPOINTS.content.base(roomName)
      }?reservation_id=${prepareResponse.reservation_id}`,
      formData,
      { token: authToken },
    );

    const uploadedContents = response.uploaded;

    if (uploadedContents.length === 0) {
      throw new Error("Failed to upload message");
    }

    return {
      id: String(uploadedContents[0].id),
      content: contentString,
      timestamp: uploadedContents[0].created_at,
      isOwn: true,
    };
  } catch (error) {
    console.error("postMessage fallback (optimistic):", error);
    const now = new Date().toISOString();
    return {
      id: `temp-${now}`,
      content,
      timestamp: now,
      isOwn: true,
    };
  }
}

/**
 * Delete a message
 *
 * @param roomName - The name of the room
 * @param messageId - The ID of the message to delete
 * @param token - Optional token for authentication
 */
export async function deleteMessage(
  roomName: string,
  messageId: string,
  token?: string,
): Promise<void> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to delete messages");
  }

  // Backend expects token in query parameter AND ids in both query and body
  await api.delete(
    `${
      API_ENDPOINTS.content.base(roomName)
    }?ids=${messageId}&token=${authToken}`,
    { ids: [parseInt(messageId, 10)] },
  );
}

/**
 * Delete multiple messages
 *
 * @param roomName - The name of the room
 * @param messageIds - Array of message IDs to delete
 * @param token - Optional token for authentication
 */
export async function deleteMessages(
  roomName: string,
  messageIds: string[],
  token?: string,
): Promise<void> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to delete messages");
  }

  const idsParam = messageIds.join(",");
  // Backend expects token in query parameter AND ids in both query and body
  await api.delete(
    `${
      API_ENDPOINTS.content.base(roomName)
    }?ids=${idsParam}&token=${authToken}`,
    { ids: messageIds.map((id) => parseInt(id, 10)) },
  );
}

/**
 * Update a message
 *
 * Uses the backend's update_content API (PUT /api/v1/rooms/{name}/contents/{content_id})
 *
 * @param roomName - The name of the room
 * @param messageId - The ID of the message to update
 * @param content - The new message content
 * @param token - Optional token for authentication
 * @returns The updated message
 */
export async function updateMessage(
  roomName: string,
  messageId: string,
  content: string,
  token?: string,
): Promise<Message> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to update messages");
  }

  // Call the update_content API
  const response = await api.put<{ updated: BackendRoomContent }>(
    API_ENDPOINTS.content.byId(roomName, messageId),
    { text: content },
    { token: authToken },
  );

  return convertMessage(response.updated);
}

export default {
  getMessages,
  postMessage,
  updateMessage,
  deleteMessage,
  deleteMessages,
};
