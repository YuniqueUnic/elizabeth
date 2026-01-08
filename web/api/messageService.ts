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
  BackendRoomContent,
  CreateMessageResponse,
  Message,
  UpdateContentResponse,
} from "../lib/types";
import {
  backendContentToMessage as convertMessage,
  ContentType as CT,
  parseContentType,
} from "../lib/types";

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

  // Filter only ContentType.Text (content_type = 0)
  const filteredContents = contents.filter((content) => {
    const contentType = parseContentType(content.content_type);
    return contentType === CT.Text;
  });

  return filteredContents
    .map(convertMessage)
    .sort((a, b) =>
      new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
}

/**
 * Send a message to a room
 *
 * Uses the new message creation API (POST /api/v1/rooms/{name}/messages)
 * Messages are stored as ContentType.Text in the database
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

  const contentString = typeof content === "string"
    ? content
    : String(content);

  const response = await api.post<CreateMessageResponse>(
    API_ENDPOINTS.content.messages(roomName),
    { text: contentString },
    { token: authToken },
  );

  const message = response.message;

  return {
    id: String(message.id),
    content: contentString,
    timestamp: message.created_at,
    isOwn: true,
  };
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
  const response = await api.put<UpdateContentResponse>(
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
