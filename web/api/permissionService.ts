/**
 * Permission Management Service
 *
 * This service handles room permission management including:
 * - Setting room permissions
 * - Checking user permissions
 * - Managing room access based on permissions
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import { encodePermissions, parsePermissions } from "../lib/types";
import type { RoomDetails, RoomPermission } from "../lib/types";

// ============================================================================
// Permission Service Types
// ============================================================================

export interface SetPermissionsRequest {
  permissions: RoomPermission[];
  makePermanent?: boolean; // Whether to make these permissions permanent for the room
}

export interface PermissionCheck {
  hasPermission: boolean;
  permissions: RoomPermission[];
  isPermanent: boolean;
  canRead: boolean;
  canEdit: boolean;
  canShare: boolean;
  canDelete: boolean;
}

export interface RoomAccessRequest {
  roomName: string;
  password?: string;
  skipCache?: boolean;
}

// ============================================================================
// Permission Management Functions
// ============================================================================

/**
 * Set permissions for a room
 * Only the room creator (admin) can set permissions and only once
 *
 * @param roomName - The name of the room
 * @param permissions - Array of permissions to set
 * @param makePermanent - Whether to make permissions permanent (default: true)
 * @param token - Optional token for authentication
 */
export async function setRoomPermissions(
  roomName: string,
  permissions: RoomPermission[],
  makePermanent: boolean = true,
  token?: string,
): Promise<void> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to set room permissions");
  }

  const permissionBits = encodePermissions(permissions);

  await api.post(
    API_ENDPOINTS.rooms.permissions(roomName),
    {
      permission: permissionBits,
      make_permanent: makePermanent,
    },
    { token: authToken },
  );
}

/**
 * Get current user permissions for a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Permission check result
 */
export async function getUserPermissions(
  roomName: string,
  token?: string,
): Promise<PermissionCheck> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    // No token means no permissions
    return {
      hasPermission: false,
      permissions: [],
      isPermanent: false,
      canRead: false,
      canEdit: false,
      canShare: false,
      canDelete: false,
    };
  }

  try {
    // Get room details to check permissions
    const room = await api.get(
      API_ENDPOINTS.rooms.base(roomName),
      undefined,
      { token: authToken },
    );

    const permissions = parsePermissions(room.permission);
    const hasPermission = permissions.length > 0;

    return {
      hasPermission,
      permissions,
      isPermanent: room.permissions_permanent || false,
      canRead: permissions.includes("read"),
      canEdit: permissions.includes("edit"),
      canShare: permissions.includes("share"),
      canDelete: permissions.includes("delete"),
    };
  } catch (error) {
    // If we can't get room details, assume no permissions
    return {
      hasPermission: false,
      permissions: [],
      isPermanent: false,
      canRead: false,
      canEdit: false,
      canShare: false,
      canDelete: false,
    };
  }
}

/**
 * Check if user has specific permission for a room
 *
 * @param roomName - The name of the room
 * @param permission - The permission to check
 * @param token - Optional token for authentication
 * @returns Whether user has the permission
 */
export async function hasPermission(
  roomName: string,
  permission: RoomPermission,
  token?: string,
): Promise<boolean> {
  const permissionCheck = await getUserPermissions(roomName, token);
  return permissionCheck.permissions.includes(permission);
}

/**
 * Check if user can access a room (has read permission)
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Whether user can access the room
 */
export async function canAccessRoom(
  roomName: string,
  token?: string,
): Promise<boolean> {
  return await hasPermission(roomName, "read", token);
}

/**
 * Check if user can edit content in a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Whether user can edit content
 */
export async function canEditContent(
  roomName: string,
  token?: string,
): Promise<boolean> {
  return await hasPermission(roomName, "edit", token);
}

/**
 * Check if user can share a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Whether user can share the room
 */
export async function canShareRoom(
  roomName: string,
  token?: string,
): Promise<boolean> {
  return await hasPermission(roomName, "share", token);
}

/**
 * Check if user can delete content in a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Whether user can delete content
 */
export async function canDeleteContent(
  roomName: string,
  token?: string,
): Promise<boolean> {
  return await hasPermission(roomName, "delete", token);
}

/**
 * Generate shareable link for a room
 * Only available if user has share permission
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Shareable room URL
 */
export async function generateShareableLink(
  roomName: string,
  token?: string,
): Promise<string> {
  if (!(await canShareRoom(roomName, token))) {
    throw new Error("You don't have permission to share this room");
  }

  // Return the room URL (this will be handled by routing)
  return `/${roomName}`;
}

/**
 * Validate if a room name is shareable (has SHARE permission)
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Whether room is shareable
 */
export async function isRoomShareable(
  roomName: string,
  token?: string,
): Promise<boolean> {
  try {
    const room = await api.get(
      API_ENDPOINTS.rooms.base(roomName),
      undefined,
      { token: token || (await getValidToken(roomName)) || undefined },
    );

    // Check if room has SHARE permission (bit 2 = 4)
    return (room.permission & 4) !== 0;
  } catch (error) {
    return false;
  }
}

// ============================================================================
// Default Permission Presets
// ============================================================================

export const PERMISSION_PRESETS = {
  READ_ONLY: ["read"] as RoomPermission[],
  READ_WRITE: ["read", "edit"] as RoomPermission[],
  READ_WRITE_SHARE: ["read", "edit", "share"] as RoomPermission[],
  FULL_ACCESS: ["read", "edit", "share", "delete"] as RoomPermission[],
} as const;

/**
 * Get description for permission preset
 */
export function getPermissionDescription(
  permissions: RoomPermission[],
): string {
  if (permissions.length === 0) return "无权限";
  if (permissions.includes("delete")) return "完全访问（读、写、分享、删除）";
  if (permissions.includes("share")) return "读写分享（读、写、分享）";
  if (permissions.includes("edit")) return "读写权限（读、写）";
  if (permissions.includes("read")) return "只读权限";
  return "自定义权限";
}

/**
 * Get color for permission level
 */
export function getPermissionColor(permissions: RoomPermission[]): string {
  if (permissions.length === 0) return "destructive";
  if (permissions.includes("delete")) return "default";
  if (permissions.includes("share")) return "secondary";
  if (permissions.includes("edit")) return "outline";
  return "ghost";
}

// ============================================================================
// Permission Utilities
// ============================================================================

/**
 * Create permission-aware API wrapper
 * Returns a function that will check permissions before making API calls
 */
export function createPermissionAwareAPI<
  T extends (...args: any[]) => Promise<any>,
>(
  apiCall: T,
  permissionCheck: () => Promise<boolean>,
  errorMessage: string,
): T {
  return (async (...args: Parameters<T>) => {
    const hasPermission = await permissionCheck();
    if (!hasPermission) {
      throw new Error(errorMessage);
    }
    return apiCall(...args);
  }) as T;
}

export default {
  setRoomPermissions,
  getUserPermissions,
  hasPermission,
  canAccessRoom,
  canEditContent,
  canShareRoom,
  canDeleteContent,
  generateShareableLink,
  isRoomShareable,
  PERMISSION_PRESETS,
  getPermissionDescription,
  getPermissionColor,
  createPermissionAwareAPI,
};
