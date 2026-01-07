/**
 * Room Management Service
 *
 * This service handles room-related operations including:
 * - Creating rooms
 * - Fetching room details
 * - Updating room permissions
 * - Deleting rooms
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type {
  BackendRoom,
  RoomDetails,
  RoomPermission,
  RoomTokenView,
  UpdateRoomPermissionRequest,
  UpdateRoomSettingsRequest,
} from "../lib/types";
import { backendRoomToRoomDetails as convertRoom } from "../lib/types";

// ============================================================================
// Room Management Functions
// ============================================================================

/**
 * Create a new room
 *
 * @param name - The name of the room
 * @param password - Optional password for the room
 * @returns Room details
 */
export async function createRoom(
  name: string,
  password?: string,
): Promise<RoomDetails> {
  // Build URL with password query parameter if provided
  let url = API_ENDPOINTS.rooms.base(name);
  if (password) {
    url += `?password=${encodeURIComponent(password)}`;
  }

  const room = await api.post<BackendRoom>(
    url,
    null,
    { skipTokenInjection: true },
  );

  return convertRoom(room);
}

/**
 * Get room details
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @param skipAuth - If true, skip token requirement (for checking if room exists)
 * @returns Room details
 */
export async function getRoomDetails(
  roomName: string,
  token?: string,
  skipAuth?: boolean,
): Promise<RoomDetails> {
  let authToken: string | undefined;

  if (!skipAuth) {
    authToken = token || (await getValidToken(roomName)) || undefined;
  }

  const room = await api.get<BackendRoom>(
    API_ENDPOINTS.rooms.base(roomName),
    undefined,
    { token: authToken, skipTokenInjection: skipAuth },
  );

  return convertRoom(room);
}

/**
 * Delete a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 */
export async function deleteRoom(
  roomName: string,
  token?: string,
): Promise<void> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to delete room");
  }

  await api.delete(
    API_ENDPOINTS.rooms.base(roomName),
    undefined,
    { token: authToken },
  );
}

/**
 * Update room permissions
 *
 * @param roomName - The name of the room
 * @param permissions - Array of permissions to set
 * @param token - Optional token for authentication
 * @returns Updated room details
 */
export async function updateRoomPermissions(
  roomName: string,
  permissions: RoomPermission[],
  token?: string,
): Promise<RoomDetails> {
  const authToken = token || (await getValidToken(roomName));

  if (!authToken) {
    throw new Error("Authentication required to update permissions");
  }

  // Backend expects { edit: bool, share: bool, delete: bool }
  // VIEW_ONLY (read) is always included by default
  const payload: UpdateRoomPermissionRequest = {
    edit: permissions.includes("edit"),
    share: permissions.includes("share"),
    delete: permissions.includes("delete"),
  };
  const room = await api.post<BackendRoom>(
    API_ENDPOINTS.rooms.permissions(roomName),
    payload,
    { token: authToken },
  );
  return convertRoom(room);
}

/**
 * Update room settings
 *
 * @param roomName - The name of the room
 * @param settings - Room settings to update
 * @param token - Optional token for authentication
 * @returns Updated room details
 */
export async function updateRoomSettings(
  roomName: string,
  settings: {
    password?: string | null;
    expiresAt?: string | null;
    maxViews?: number;
    maxSize?: number;
  },
  token?: string,
): Promise<RoomDetails> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to update room settings");
  }

  // Convert frontend settings to backend format
  const payload: UpdateRoomSettingsRequest = {};

  if (settings.password !== undefined) {
    // ✅ FIX: Send empty string to clear password, backend expects empty string not null
    payload.password = settings.password === null ? "" : settings.password;
  }

  if (settings.expiresAt !== undefined) {
    payload.expire_at = settings.expiresAt ?? undefined;
  }

  if (settings.maxViews !== undefined) {
    payload.max_times_entered = settings.maxViews;
  }

  if (settings.maxSize !== undefined) {
    payload.max_size = settings.maxSize;
  }

  const room = await api.put<BackendRoom>(
    API_ENDPOINTS.rooms.settings(roomName),
    payload,
    { token: authToken },
  );

  return convertRoom(room);
}

/**
 * List all tokens for a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns List of tokens
 */
export async function listRoomTokens(
  roomName: string,
  token?: string,
): Promise<RoomTokenView[]> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to list tokens");
  }

  return api.get<RoomTokenView[]>(
    API_ENDPOINTS.rooms.tokens(roomName),
    undefined,
    { token: authToken },
  );
}

// Legacy compatibility exports (for existing components)
// getRoomDetails is already exported above

export default {
  createRoom,
  getRoomDetails,
  deleteRoom,
  updateRoomPermissions,
  listRoomTokens,
};
