/**
 * Unified API Request Handler
 *
 * This module provides a unified interface for making API requests to the backend,
 * including automatic token injection, error handling, and response normalization.
 */

import { API_BASE_URL, REQUEST_CONFIG, TOKEN_CONFIG } from "../config";
import type { TokenInfo, TokenStorage } from "../types";

// ============================================================================
// Token Management
// ============================================================================

/**
 * Get all stored tokens from localStorage
 */
export function getStoredTokens(): TokenStorage {
  if (typeof window === "undefined") return {};

  try {
    const stored = localStorage.getItem(TOKEN_CONFIG.storageKey);
    return stored ? JSON.parse(stored) : {};
  } catch (error) {
    console.error("Failed to parse stored tokens:", error);
    return {};
  }
}

/**
 * Save tokens to localStorage
 */
export function saveTokens(tokens: TokenStorage): void {
  if (typeof window === "undefined") return;

  try {
    localStorage.setItem(TOKEN_CONFIG.storageKey, JSON.stringify(tokens));
  } catch (error) {
    console.error("Failed to save tokens:", error);
  }
}

/**
 * Get token for a specific room
 */
export function getRoomToken(roomName: string): TokenInfo | null {
  const tokens = getStoredTokens();
  return tokens[roomName] || null;
}

/**
 * Get the raw JWT token string for a room
 */
export function getRoomTokenString(roomName: string): string | null {
  const tokenInfo = getRoomToken(roomName);
  return tokenInfo?.token || null;
}

/**
 * Set token for a specific room
 */
export function setRoomToken(roomName: string, tokenInfo: TokenInfo): void {
  const tokens = getStoredTokens();
  tokens[roomName] = tokenInfo;
  saveTokens(tokens);
}

/**
 * Clear token for a specific room
 */
export function clearRoomToken(roomName: string): void {
  const tokens = getStoredTokens();
  delete tokens[roomName];
  saveTokens(tokens);
}

/**
 * Clear all tokens
 */
export function clearAllTokens(): void {
  if (typeof window === "undefined") return;
  localStorage.removeItem(TOKEN_CONFIG.storageKey);
}

/**
 * Check if a token is expired or will expire soon
 */
export function isTokenExpired(
  expiresAt: string,
  bufferMs: number = TOKEN_CONFIG.refreshBeforeExpiry,
): boolean {
  // Some backends may return ISO timestamps without timezone information (e.g. "2025-10-29T08:47:40").
  // In JS, such strings are interpreted as local time, which can incorrectly mark tokens as expired
  // when they were intended to be UTC. To be robust, treat timestamps lacking timezone as UTC.
  const hasTimezone = /[zZ]|[+-]\d{2}:?\d{2}$/.test(expiresAt);
  const normalized = hasTimezone ? expiresAt : `${expiresAt}Z`;
  const expiryTime = new Date(normalized).getTime();
  const now = Date.now();
  return (expiryTime - now) <= bufferMs;
}

/**
 * Try to get a fresh token for a room (for non-password-protected rooms)
 */
export async function refreshRoomToken(
  roomName: string,
): Promise<string | null> {
  try {
    const response = await fetch(
      `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({}),
      },
    );

    if (!response.ok) {
      return null;
    }

    const data = await response.json();
    if (data.token) {
      const tokenInfo: TokenInfo = {
        token: data.token,
        expiresAt: data.expires_at,
      };

      setRoomToken(roomName, tokenInfo);
      return data.token;
    }

    return null;
  } catch (error) {
    console.error("Error refreshing token for room:", roomName, error);
    return null;
  }
}

// ============================================================================
// API Response Types
// ============================================================================

export interface APIResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  code?: number;
  message?: string;
  timestamp?: string;
}

export class APIError extends Error {
  constructor(
    message: string,
    public code?: number,
    public response?: Response,
  ) {
    super(message);
    this.name = "APIError";
  }
}

// ============================================================================
// Request Configuration
// ============================================================================

export interface RequestOptions extends RequestInit {
  token?: string;
  skipTokenInjection?: boolean;
  retries?: number;
  retryDelay?: number;
  responseType?: "json" | "text" | "blob";
}

/**
 * Build full URL with query parameters
 */
function buildURL(
  path: string,
  params?: Record<string, string | number | boolean>,
): string {
  const url = new URL(
    path.startsWith("http") ? path : `${API_BASE_URL}${path}`,
  );

  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        url.searchParams.append(key, String(value));
      }
    });
  }

  return url.toString();
}

/**
 * Inject token into request
 */
function injectToken(url: string, token?: string, roomName?: string): string {
  // If token is explicitly provided, use it
  if (token) {
    const urlObj = new URL(url);
    urlObj.searchParams.set("token", token);
    return urlObj.toString();
  }

  // Try to get token from storage if roomName is provided
  if (roomName) {
    const tokenInfo = getRoomToken(roomName);
    if (tokenInfo && !isTokenExpired(tokenInfo.expiresAt)) {
      const urlObj = new URL(url);
      urlObj.searchParams.set("token", tokenInfo.token);
      return urlObj.toString();
    }
  }

  return url;
}

/**
 * Parse API response
 */
async function parseResponse<T>(
  response: Response,
  responseType: string = "json",
): Promise<T> {
  // Handle different response types
  switch (responseType) {
    case "blob":
      return await response.blob() as unknown as T;

    case "text":
      return await response.text() as unknown as T;

    case "json":
    default:
      const contentType = response.headers.get("content-type");

      if (contentType?.includes("application/json")) {
        const json = await response.json();

        // Backend returns: { code, success, message, data?, timestamp? }
        if (json.success === false) {
          throw new APIError(
            json.message || "Request failed",
            json.code || response.status,
            response,
          );
        }

        // Return the data field if it exists, otherwise return the whole response
        return json.data !== undefined ? json.data : json;
      }

      // For non-JSON responses, return text
      const text = await response.text();
      return text as unknown as T;
  }
}

// ============================================================================
// Core Request Functions
// ============================================================================

/**
 * Make an API request with automatic retry and error handling
 */
async function request<T = any>(
  path: string,
  options: RequestOptions = {},
): Promise<T> {
  const {
    token,
    skipTokenInjection = false,
    retries = REQUEST_CONFIG.retries,
    retryDelay = REQUEST_CONFIG.retryDelay,
    responseType = "json",
    ...fetchOptions
  } = options;

  let url = buildURL(path);

  // Extract room name from path for token refresh logic
  const roomMatch = path.match(/\/rooms\/([^\/]+)/);
  const roomName = roomMatch ? decodeURIComponent(roomMatch[1]) : null;

  // Inject token if not skipped
  if (!skipTokenInjection && token) {
    url = injectToken(url, token);
  } else if (!skipTokenInjection && roomName && !token) {
    // Try to get token from storage for this room
    url = injectToken(url, undefined, roomName);
  }

  // Set default headers
  const headers = new Headers(fetchOptions.headers);
  if (
    !headers.has("Content-Type") && fetchOptions.body &&
    typeof fetchOptions.body === "string"
  ) {
    headers.set("Content-Type", "application/json");
  }

  let lastError: Error | null = null;
  let tokenRefreshAttempted = false;

  // Retry logic with token refresh on 401
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        REQUEST_CONFIG.timeout,
      );

      const response = await fetch(url, {
        ...fetchOptions,
        headers,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        // Handle 401 Unauthorized - try token refresh
        if (
          response.status === 401 && roomName && !tokenRefreshAttempted &&
          !skipTokenInjection
        ) {
          clearRoomToken(roomName);
          tokenRefreshAttempted = true;

          const freshToken = await refreshRoomToken(roomName);

          if (freshToken) {
            url = buildURL(path);
            url = injectToken(url, freshToken);
            attempt--;
            continue;
          } else {
            // Throw a more user-friendly error that the UI can handle
            const errorData = await response.json();
            throw new APIError(
              `Token expired: ${
                errorData.message ||
                "Please refresh the page to re-authenticate"
              }`,
              401,
              response,
            );
          }
        }

        // Try to parse error response
        try {
          const errorData = await response.json();
          throw new APIError(
            errorData.message || response.statusText,
            errorData.code || response.status,
            response,
          );
        } catch (parseError) {
          throw new APIError(
            response.statusText || "Request failed",
            response.status,
            response,
          );
        }
      }

      return await parseResponse<T>(response, responseType);
    } catch (error) {
      lastError = error as Error;

      // Don't retry on client errors (4xx) or abort errors, except for 401 which we handled above
      if (
        error instanceof APIError && error.code && error.code >= 400 &&
        error.code < 500 && error.code !== 401
      ) {
        throw error;
      }

      if (error instanceof Error && error.name === "AbortError") {
        throw new APIError("Request timeout", 408);
      }

      // Wait before retrying (except on last attempt)
      if (attempt < retries) {
        await new Promise((resolve) =>
          setTimeout(resolve, retryDelay * (attempt + 1))
        );
      }
    }
  }

  throw lastError || new APIError("Request failed after retries");
}

// ============================================================================
// HTTP Method Helpers
// ============================================================================

export const api = {
  /**
   * Make a GET request
   */
  get: <T = any>(
    path: string,
    params?: Record<string, string | number | boolean>,
    options?: RequestOptions,
  ): Promise<T> => {
    const url = params ? buildURL(path, params) : path;
    return request<T>(url, { ...options, method: "GET" });
  },

  /**
   * Make a POST request
   */
  post: <T = any>(
    path: string,
    data?: any,
    options?: RequestOptions,
  ): Promise<T> => {
    return request<T>(path, {
      ...options,
      method: "POST",
      body: data instanceof FormData ? data : JSON.stringify(data),
    });
  },

  /**
   * Make a PUT request
   */
  put: <T = any>(
    path: string,
    data?: any,
    options?: RequestOptions,
  ): Promise<T> => {
    return request<T>(path, {
      ...options,
      method: "PUT",
      body: JSON.stringify(data),
    });
  },

  /**
   * Make a DELETE request
   */
  delete: <T = any>(
    path: string,
    data?: any,
    options?: RequestOptions,
  ): Promise<T> => {
    return request<T>(path, {
      ...options,
      method: "DELETE",
      body: data ? JSON.stringify(data) : undefined,
    });
  },
};

export default api;
