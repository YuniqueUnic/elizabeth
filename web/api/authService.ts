/**
 * Authentication Service
 *
 * This service handles JWT token management, including:
 * - Obtaining access tokens
 * - Validating tokens
 * - Refreshing tokens
 * - Logging out (revoking tokens)
 */

import { API_ENDPOINTS, TOKEN_CONFIG } from "../lib/config";
import {
  api,
  clearRoomToken,
  getRoomToken,
  isTokenExpired,
  setRoomToken,
} from "../lib/utils/api";
import type {
  BackendTokenResponse,
  BackendTokenValidation,
  TokenInfo,
} from "../lib/types";

// ============================================================================
// Token Request/Response Types
// ============================================================================

export interface IssueTokenRequest {
  password?: string;
  permission?: number;
  ttl_seconds?: number;
  max_uses?: number;
  expires_at?: string;
}

export interface IssueTokenResponse {
  token: string;
  jti: string;
  permission: number;
  room_name: string;
  max_uses: number | null;
  uses: number;
  expires_at: string;
  created_at: string;
}

export interface RefreshTokenRequest {
  refresh_token: string;
}

export interface RefreshTokenResponse extends BackendTokenResponse {
  // Same as BackendTokenResponse
}

// ============================================================================
// Authentication Functions
// ============================================================================

/**
 * Get an access token for a room
 *
 * @param roomName - The name of the room
 * @param password - Optional password for the room
 * @param options - Additional token options (permission, ttl, max_uses, expires_at)
 * @returns Token information including the JWT token
 */
export async function getAccessToken(
  roomName: string,
  password?: string,
  options?: Omit<IssueTokenRequest, "password">,
): Promise<IssueTokenResponse> {
  const requestBody: IssueTokenRequest = {
    password,
    ...options,
  };

  const response = await api.post<IssueTokenResponse>(
    API_ENDPOINTS.rooms.tokens(roomName),
    requestBody,
    { skipTokenInjection: true },
  );

  const tokenInfo: TokenInfo = {
    token: response.token,
    expiresAt: response.expires_at,
  };

  setRoomToken(roomName, tokenInfo);

  return response;
}

/**
 * Validate a token for a room
 *
 * @param roomName - The name of the room
 * @param token - Optional token to validate (uses stored token if not provided)
 * @returns Validation result
 */
export async function validateToken(
  roomName: string,
  token?: string,
): Promise<BackendTokenValidation> {
  const tokenToValidate = token || getRoomToken(roomName)?.token;

  if (!tokenToValidate) {
    throw new Error("No token available for validation");
  }

  return api.post<BackendTokenValidation>(
    API_ENDPOINTS.rooms.validateToken(roomName),
    { token: tokenToValidate },
    { skipTokenInjection: true },
  );
}

/**
 * Refresh an access token using a refresh token
 *
 * @param refreshToken - The refresh token
 * @returns New token information
 */
export async function refreshToken(
  refreshToken: string,
): Promise<RefreshTokenResponse> {
  const response = await api.post<RefreshTokenResponse>(
    API_ENDPOINTS.auth.refresh,
    { refresh_token: refreshToken },
    { skipTokenInjection: true },
  );

  // Note: We can't determine which room this token is for from the response
  // The calling code should handle storing the token with the appropriate room name

  return response;
}

/**
 * Log out by revoking the access token
 *
 * @param accessToken - The access token to revoke
 */
export async function logout(accessToken?: string): Promise<void> {
  await api.post(
    API_ENDPOINTS.auth.logout,
    { access_token: accessToken },
    { skipTokenInjection: true },
  );
}

/**
 * Revoke a specific room token
 *
 * @param roomName - The name of the room
 * @param jti - The token ID (jti) to revoke
 * @param token - Optional admin/access token
 */
export async function revokeRoomToken(
  roomName: string,
  jti: string,
  token?: string,
): Promise<void> {
  await api.delete(
    API_ENDPOINTS.rooms.revokeToken(roomName, jti),
    { token },
  );

  // If this was the currently stored token, clear it
  const currentToken = getRoomToken(roomName);
  if (currentToken) {
    clearRoomToken(roomName);
  }
}

/**
 * Get the current valid token for a room, refreshing if necessary
 *
 * @param roomName - The name of the room
 * @returns Valid token or null if no token available
 */
export async function getValidToken(roomName: string): Promise<string | null> {
  const tokenInfo = getRoomToken(roomName);

  if (!tokenInfo) {
    return null;
  }

  if (!tokenInfo.token || !tokenInfo.expiresAt) {
    return null;
  }

  // Check if token needs refresh
  if (isTokenExpired(tokenInfo.expiresAt)) {
    if (tokenInfo.refreshToken) {
      try {
        const newTokenInfo = await refreshToken(tokenInfo.refreshToken);
        const updatedToken: TokenInfo = {
          token: newTokenInfo.token,
          expiresAt: newTokenInfo.expires_at,
          refreshToken: newTokenInfo.refresh_token,
        };
        setRoomToken(roomName, updatedToken);
        return newTokenInfo.token;
      } catch (error) {
        console.error("Failed to refresh token:", error);
        clearRoomToken(roomName);
        return null;
      }
    } else {
      clearRoomToken(roomName);
      return null;
    }
  }

  return tokenInfo.token;
}

/**
 * Check if user has a valid token for a room
 *
 * @param roomName - The name of the room
 * @returns True if user has a valid token
 */
export function hasValidToken(roomName: string): boolean {
  const tokenInfo = getRoomToken(roomName);
  if (!tokenInfo) return false;

  // Check if token is not expired (with buffer)
  return !isTokenExpired(tokenInfo.expiresAt);
}

export default {
  getAccessToken,
  validateToken,
  refreshToken,
  logout,
  revokeRoomToken,
  getValidToken,
  hasValidToken,
};
