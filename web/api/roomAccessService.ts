/**
 * Room Access Management Service
 *
 * This service handles room access management including:
 * - Password-protected room access
 * - UUID-based access for non-shareable rooms
 * - Room availability checks
 * - Access token management
 */

import { API_ENDPOINTS } from "../lib/config";
import { api, getRoomToken, saveTokens } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type { RoomDetails } from "../lib/types";

// ============================================================================
// Room Access Types
// ============================================================================

export interface RoomAccessResult {
  success: boolean;
  roomDetails?: RoomDetails;
  token?: string;
  requiresPassword?: boolean;
  error?: string;
  isAccessible?: boolean;
}

export interface RoomAvailability {
  exists: boolean;
  accessible: boolean;
  expired?: boolean;
  full?: boolean;
  requiresPassword?: boolean;
  isShareable?: boolean;
}

export interface RoomAccessOptions {
  password?: string;
  skipCache?: boolean;
  validateOnly?: boolean; // Only check access, don't get token
}

// ============================================================================
// Room Access Functions
// ============================================================================

/**
 * Check if a room exists and is accessible
 *
 * @param roomName - The name of the room
 * @returns Room availability information
 */
export async function checkRoomAvailability(
  roomName: string,
): Promise<RoomAvailability> {
  try {
    // Try to get room details without authentication
    const room = await api.get(API_ENDPOINTS.rooms.base(roomName), undefined, {
      skipTokenInjection: true,
    });

    const now = new Date();
    const expiresAt = room.expire_at ? new Date(room.expire_at) : null;
    const isExpired = expiresAt ? expiresAt <= now : false;
    const isFull = room.current_times_entered >= room.max_times_entered;

    return {
      exists: true,
      accessible: !isExpired && !isFull,
      expired: isExpired,
      full: isFull,
      requiresPassword: room.password !== null && room.password !== "",
      isShareable: (room.permission & 4) !== 0, // SHARE permission bit
    };
  } catch (error: any) {
    if (error.code === 404) {
      return {
        exists: false,
        accessible: false,
      };
    }
    throw error;
  }
}

/**
 * Generate UUID for non-shareable rooms
 */
export function generateRoomUUID(): string {
  return crypto.randomUUID().replace(/-/g, "").substring(0, 8);
}

/**
 * Create room name with UUID for non-shareable rooms
 *
 * @param baseName - The base room name
 * @param roomDetails - Room details to check shareability
 * @returns Room name (with UUID if needed)
 */
export async function getAccessibleRoomName(
  baseName: string,
  roomDetails?: RoomDetails,
): Promise<string> {
  // If room details are provided and room is shareable, return base name
  if (
    roomDetails && (roomDetails.permissions.includes("share") ||
      (roomDetails as any).permissionBits?.includes(4))
  ) {
    return baseName;
  }

  // Check if base name is available
  try {
    const availability = await checkRoomAvailability(baseName);
    if (!availability.exists) {
      return baseName;
    }
  } catch (error) {
    // If we can't check, assume base name is not available
  }

  // Generate UUID and try to create unique room name
  let attempts = 0;
  const maxAttempts = 5;

  while (attempts < maxAttempts) {
    const uuid = generateRoomUUID();
    const roomNameWithUUID = `${baseName}_${uuid}`;

    try {
      const availability = await checkRoomAvailability(roomNameWithUUID);
      if (!availability.exists) {
        return roomNameWithUUID;
      }
    } catch (error) {
      // Continue trying
    }

    attempts++;
  }

  // If all attempts fail, return the last generated UUID name
  return `${baseName}_${generateRoomUUID()}`;
}

/**
 * Access a room with password
 *
 * @param roomName - The name of the room
 * @param password - The room password
 * @returns Room access result with token
 */
export async function accessRoomWithPassword(
  roomName: string,
  password: string,
): Promise<RoomAccessResult> {
  try {
    const result = await api.post<{
      token: string;
      expires_in: number;
    }>(
      API_ENDPOINTS.rooms.base(roomName),
      { password },
      { skipTokenInjection: true },
    );

    return {
      success: true,
      token: result.token,
    };
  } catch (error: any) {
    if (error.code === 401) {
      return {
        success: false,
        requiresPassword: true,
        error: "密码错误",
      };
    }
    return {
      success: false,
      error: error.message || "无法访问房间",
    };
  }
}

/**
 * Access a shareable room (no password required)
 *
 * @param roomName - The name of the room
 * @returns Room access result with token
 */
export async function accessShareableRoom(
  roomName: string,
): Promise<RoomAccessResult> {
  try {
    const result = await api.post<{
      token: string;
      expires_in: number;
    }>(
      API_ENDPOINTS.rooms.base(roomName),
      {},
      { skipTokenInjection: true },
    );

    return {
      success: true,
      token: result.token,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message || "无法访问房间",
    };
  }
}

/**
 * Complete room access process
 * Handles password validation, token generation, and room details retrieval
 *
 * @param roomName - The name of the room
 * @param options - Access options
 * @returns Complete room access result
 */
export async function accessRoom(
  roomName: string,
  options: RoomAccessOptions = {},
): Promise<RoomAccessResult> {
  const { password, skipCache = false, validateOnly = false } = options;

  // Check cache first (unless skipped)
  if (!skipCache) {
    const existingToken = getRoomToken(roomName);
    if (existingToken && !isTokenExpired(existingToken)) {
      try {
        const roomDetails = await getRoomDetailsWithToken(
          roomName,
          existingToken.token,
        );
        return {
          success: true,
          roomDetails,
          token: existingToken.token,
          isAccessible: true,
        };
      } catch (error) {
        // Token is invalid, continue with normal flow
      }
    }
  }

  // Check room availability
  const availability = await checkRoomAvailability(roomName);

  if (!availability.exists) {
    return {
      success: false,
      error: "房间不存在",
    };
  }

  if (!availability.accessible) {
    let errorMessage = "房间无法访问";
    if (availability.expired) errorMessage = "房间已过期";
    if (availability.full) errorMessage = "房间访问次数已达上限";

    return {
      success: false,
      error: errorMessage,
    };
  }

  // Handle password-protected rooms
  if (availability.requiresPassword) {
    if (!password) {
      return {
        success: false,
        requiresPassword: true,
        error: "需要密码才能访问",
      };
    }

    const passwordResult = await accessRoomWithPassword(roomName, password);
    if (!passwordResult.success) {
      return passwordResult;
    }

    if (validateOnly) {
      return {
        success: true,
        isAccessible: true,
      };
    }

    // Get room details with the new token
    const roomDetails = await getRoomDetailsWithToken(
      roomName,
      passwordResult.token!,
    );

    // Save token to cache
    saveTokens({
      ...getStoredTokens(),
      [roomName]: {
        token: passwordResult.token!,
        expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(), // 30 minutes
        refreshToken: passwordResult.token,
      },
    });

    return {
      success: true,
      roomDetails,
      token: passwordResult.token,
      isAccessible: true,
    };
  }

  // Handle shareable rooms (no password required)
  if (availability.isShareable) {
    const shareableResult = await accessShareableRoom(roomName);
    if (!shareableResult.success) {
      return shareableResult;
    }

    if (validateOnly) {
      return {
        success: true,
        isAccessible: true,
      };
    }

    // Get room details with the new token
    const roomDetails = await getRoomDetailsWithToken(
      roomName,
      shareableResult.token!,
    );

    // Save token to cache
    saveTokens({
      ...getStoredTokens(),
      [roomName]: {
        token: shareableResult.token!,
        expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(), // 30 minutes
        refreshToken: shareableResult.token,
      },
    });

    return {
      success: true,
      roomDetails,
      token: shareableResult.token,
      isAccessible: true,
    };
  }

  // Non-shareable rooms should be accessed via UUID links
  // This should be handled by the routing layer
  return {
    success: false,
    error: "该房间不允许直接访问，请使用完整的链接",
  };
}

/**
 * Get room details with authentication token
 *
 * @param roomName - The name of the room
 * @param token - Authentication token
 * @returns Room details
 */
async function getRoomDetailsWithToken(
  roomName: string,
  token: string,
): Promise<RoomDetails> {
  const room = await api.get(API_ENDPOINTS.rooms.base(roomName), undefined, {
    token,
  });

  return {
    id: room.name,
    name: room.name,
    currentSize: room.current_size,
    maxSize: room.max_size,
    timesEntered: room.current_times_entered,
    maxTimesEntered: room.max_times_entered,
    settings: {
      expiresAt: room.expire_at,
      passwordProtected: room.password !== null && room.password !== "",
      maxViews: room.max_times_entered,
    },
    permissions: parsePermissions(room.permission),
    createdAt: room.created_at,
  };
}

/**
 * Check if a token is expired
 */
function isTokenExpired(tokenInfo: { expiresAt: string }): boolean {
  return new Date(tokenInfo.expiresAt) <= new Date();
}

/**
 * Get stored tokens (helper function)
 */
function getStoredTokens() {
  if (typeof window === "undefined") return {};

  try {
    const stored = localStorage.getItem("elizabeth_tokens");
    return stored ? JSON.parse(stored) : {};
  } catch (error) {
    console.error("Failed to parse stored tokens:", error);
    return {};
  }
}

export default {
  checkRoomAvailability,
  generateRoomUUID,
  getAccessibleRoomName,
  accessRoomWithPassword,
  accessShareableRoom,
  accessRoom,
};
