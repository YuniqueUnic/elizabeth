/**
 * Message Service
 *
 * This service handles chat message operations using the Content API.
 * Messages are stored as RoomContent with content_type = ContentType.Text (0)
 */

import { API_ENDPOINTS } from "../lib/config";
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
} from "../lib/types";

// ============================================================================
// Message Request/Response Types
// ============================================================================

export interface PrepareUploadRequest {
  files: Array<{
    name: string;
    size: number;
    mime: string;
  }>;
}

export interface PrepareUploadResponse {
  reservation_id: string;
  reservations: Array<{
    file_name: string;
    expected_size: number;
    mime_type: string;
  }>;
}

// ============================================================================
// Message Functions
// ============================================================================

/**
 * Get all messages for a room
 * Filters RoomContent for text-only content (content_type = 0)
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Array of messages
 */
export async function getMessages(
  roomName: string,
  token?: string,
): Promise<Message[]> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to get messages");
  }

  const contents = await api.get<BackendRoomContent[]>(
    API_ENDPOINTS.content.base(roomName),
    undefined,
    { token: authToken },
  );

  // Filter for text content only and convert to messages
  return contents
    .filter((content) => content.content_type === CT.Text)
    .map(convertMessage)
    .sort((a, b) =>
      new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
}

/**
 * Send a message to a room
 *
 * This uses a two-step process:
 * 1. Prepare upload to reserve space
 * 2. Upload the message as a text file
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

  // Step 2: Upload content
  const formData = new FormData();
  const blob = new Blob([content], { type: "text/plain" });
  formData.append("file", blob, "message.txt");

  const uploadedContents = await api.post<BackendRoomContent[]>(
    `${
      API_ENDPOINTS.content.base(roomName)
    }?reservation_id=${prepareResponse.reservation_id}`,
    formData,
    { token: authToken },
  );

  if (uploadedContents.length === 0) {
    throw new Error("Failed to upload message");
  }

  return convertMessage(uploadedContents[0]);
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

  // Backend expects an array of IDs
  await api.delete(
    `${API_ENDPOINTS.content.base(roomName)}?ids=${messageId}`,
    { token: authToken },
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
  await api.delete(
    `${API_ENDPOINTS.content.base(roomName)}?ids=${idsParam}`,
    { token: authToken },
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
