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
  backendRoomToRoomDetails,
  encodePermissions,
  RoomDetails,
  RoomPermission,
} from "../lib/types";
import {
  backendRoomToRoomDetails as convertRoom,
  encodePermissions as convertPerms,
} from "../lib/types";

// ============================================================================
// Room Request/Response Types
// ============================================================================

export interface CreateRoomRequest {
  password?: string;
}

export interface UpdatePermissionsRequest {
  permission: number;
}

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
  const queryParams: Record<string, string> = {};
  if (password) {
    queryParams.password = password;
  }

  const room = await api.post<BackendRoom>(
    API_ENDPOINTS.rooms.base(name),
    null,
    { skipTokenInjection: true },
  );

  // Add password to query if provided
  if (password) {
    const url = new URL(API_ENDPOINTS.rooms.base(name), window.location.origin);
    url.searchParams.set("password", password);

    const roomWithPassword = await api.post<BackendRoom>(
      url.pathname + url.search,
      null,
      { skipTokenInjection: true },
    );
    return convertRoom(roomWithPassword);
  }

  return convertRoom(room);
}

/**
 * Get room details
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Room details
 */
export async function getRoomDetails(
  roomName: string,
  token?: string,
): Promise<RoomDetails> {
  const authToken = token || await getValidToken(roomName);

  const room = await api.get<BackendRoom>(
    API_ENDPOINTS.rooms.base(roomName),
    undefined,
    { token: authToken || undefined },
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
    { token: authToken },
  );
}

/**
 * Update room permissions
 *
 * @param roomName - The name of the room
 * @param permissions - Array of permissions to set
 * @param token - Optional token for authentication
 */
export async function updateRoomPermissions(
  roomName: string,
  permissions: RoomPermission[],
  token?: string,
): Promise<void> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to update permissions");
  }

  const permissionBits = convertPerms(permissions);

  await api.post(
    API_ENDPOINTS.rooms.permissions(roomName),
    { permission: permissionBits },
    { token: authToken },
  );
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
  const payload: {
    password?: string | null;
    expire_at?: string | null;
    max_times_entered?: number;
    max_size?: number;
  } = {};

  if (settings.password !== undefined) {
    payload.password = settings.password;
  }

  if (settings.expiresAt !== undefined) {
    payload.expire_at = settings.expiresAt;
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
): Promise<any[]> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to list tokens");
  }

  return api.get(
    API_ENDPOINTS.rooms.tokens(roomName),
    undefined,
    { token: authToken },
  );
}

// Legacy compatibility exports (for existing components)
export { getRoomDetails };

export default {
  createRoom,
  getRoomDetails,
  deleteRoom,
  updateRoomPermissions,
  listRoomTokens,
};
