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
  MessagePage,
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
 * Get one page of messages for a room.
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @param cursor - Opaque cursor returned by the previous page
 * @param limit - Maximum page size
 */
export async function getMessagePage(
  roomName: string,
  cursor?: string | null,
  limit = 50,
  token?: string,
): Promise<MessagePage> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to get messages");
  }

  const response = await api.get<{
    items: BackendRoomContent[];
    next_cursor: string | null;
    has_more: boolean;
    next_sequence_number: number;
  }>(
    API_ENDPOINTS.content.messages(roomName),
    cursor ? { limit, cursor } : { limit },
    { token: authToken },
  );

  return {
    items: response.items
      .filter((content) => parseContentType(content.content_type) === CT.Text)
      .map(convertMessage),
    nextCursor: response.next_cursor,
    hasMore: response.has_more,
    nextSequenceNumber: response.next_sequence_number,
  };
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
  sequenceNumber?: number,
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
    { text: contentString, sequence_number: sequenceNumber },
    { token: authToken },
  );

  const message = response.message;

  return { ...convertMessage(message), isOwn: true };
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

  await api.delete(
    `${API_ENDPOINTS.content.base(roomName)}?ids=${messageId}`,
    { ids: [parseInt(messageId, 10)] },
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
    { ids: messageIds.map((id) => parseInt(id, 10)) },
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
  const response = await api.put<UpdateContentResponse>(
    API_ENDPOINTS.content.byId(roomName, messageId),
    { text: content },
    { token: authToken },
  );

  return convertMessage(response.updated);
}

const messageService = {
  getMessagePage,
  postMessage,
  updateMessage,
  deleteMessage,
  deleteMessages,
};

export default messageService;
