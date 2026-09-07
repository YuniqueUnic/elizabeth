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
  RoomGrant,
  CreateRoomRequest,
  CreateRoomResponse,
  RoomDetails,
  RoomRole,
  RoomTokenView,
  CreateRoleRequest,
  UpdateRoleRequest,
  UpdateRoomSettingsRequest,
  IssueTokenResponse,
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
  adminIdentityCode?: string,
): Promise<CreateRoomResponse> {
  const payload: CreateRoomRequest = {};
  if (password) {
    payload.password = password;
  }
  if (adminIdentityCode) {
    payload.admin_identity_code = adminIdentityCode;
  }
  const response = await api.post<CreateRoomResponse>(
    API_ENDPOINTS.rooms.base(name),
    payload,
    { skipTokenInjection: true },
  );

  return response;
}

export interface RoomIdentityCode {
  id: number;
  role: string;
  expires_at: string;
  revoked_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateRoomIdentityCodeRequest {
  code: string;
  role: string;
  expires_in_secs?: number;
}

export interface IdentityCodeResult {
  code?: string;
  identity_code: RoomIdentityCode;
}

export async function listRoomIdentityCodes(roomName: string, token?: string): Promise<RoomIdentityCode[]> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to list identity codes");
  return api.get<RoomIdentityCode[]>(API_ENDPOINTS.rooms.identityCodes(roomName), undefined, { token: authToken });
}

export async function createRoomIdentityCode(
  roomName: string,
  request: CreateRoomIdentityCodeRequest,
  token?: string,
): Promise<IdentityCodeResult> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to create identity codes");
  return api.post<IdentityCodeResult>(API_ENDPOINTS.rooms.identityCodes(roomName), request, { token: authToken });
}

export async function updateRoomIdentityCode(
  roomName: string,
  id: number,
  request: { code?: string; expires_in_secs?: number; disable?: boolean },
  token?: string,
): Promise<IdentityCodeResult> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) throw new Error("Authentication required to update identity codes");
  return api.patch<IdentityCodeResult>(API_ENDPOINTS.rooms.identityCode(roomName, id), request, { token: authToken });
}

export function redeemRoomIdentityCode(roomName: string, code: string): Promise<IssueTokenResponse> {
  return api.post<IssueTokenResponse>(API_ENDPOINTS.rooms.redeemIdentityCode(roomName), { code }, { skipTokenInjection: true });
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
 * 获取当前会话的实时能力快照（角色矩阵变更后的客户端刷新通道）。
 */
export async function getMyCapabilities(
  roomName: string,
  token?: string,
): Promise<{ role: string; capabilities: RoomGrant[] }> {
  const authToken = token || (await getValidToken(roomName)) || undefined;
  if (!authToken) throw new Error("Authentication required to read capabilities");
  return api.get<{ role: string; capabilities: RoomGrant[] }>(
    API_ENDPOINTS.rooms.capabilities(roomName),
    undefined,
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
