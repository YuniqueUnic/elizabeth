/**
 * Room Management Service
 *
 * This service handles room-related operations including:
 * - Creating rooms
 * - Fetching room details
 * - Updating room settings and role capabilities
 * - Deleting rooms
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type {
  BackendRoom,
  CreateRoomRequest,
  CreateRoomResponse,
  RoomDetails,
  RoomRole,
  RoomTokenView,
  CreateRoleRequest,
  UpdateRoleRequest,
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
): Promise<CreateRoomResponse> {
  const payload: CreateRoomRequest = {};
  if (password) {
    payload.password = password;
  }
  const response = await api.post<CreateRoomResponse>(
    API_ENDPOINTS.rooms.base(name),
    payload,
    { skipTokenInjection: true },
  );

  return response;
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

export async function listRoomRoles(roomName: string, token?: string): Promise<RoomRole[]> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to list room roles");
  return api.get<RoomRole[]>(API_ENDPOINTS.rooms.roles(roomName), undefined, { token: authToken });
}

export async function createRoomRole(roomName: string, request: CreateRoleRequest, token?: string): Promise<RoomRole> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to create room role");
  return api.post<RoomRole>(API_ENDPOINTS.rooms.roles(roomName), request, { token: authToken });
}

export async function updateRoomRole(roomName: string, roleKey: string, request: UpdateRoleRequest, token?: string): Promise<RoomRole> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to update room role");
  return api.put<RoomRole>(API_ENDPOINTS.rooms.role(roomName, roleKey), request, { token: authToken });
}

export async function deleteRoomRole(roomName: string, roleKey: string, token?: string): Promise<void> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to delete room role");
  await api.delete(API_ENDPOINTS.rooms.role(roomName, roleKey), undefined, { token: authToken });
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
    removePassword?: boolean;
    ageSeconds?: number;
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
  const payload: UpdateRoomSettingsRequest = {
    remove_password: false,
  };

  if (settings.password !== undefined) {
    payload.password = settings.password === null ? "" : settings.password;
  }

  if (settings.removePassword) {
    payload.remove_password = true;
    delete payload.password;
  }

  if (settings.ageSeconds !== undefined) {
    payload.age_seconds = settings.ageSeconds;
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

const roomService = {
  createRoom,
  getRoomDetails,
  deleteRoom,
  listRoomRoles,
  createRoomRole,
  updateRoomRole,
  deleteRoomRole,
  listRoomTokens,
};

export default roomService;
