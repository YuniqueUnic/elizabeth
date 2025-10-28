/**
 * API Configuration
 *
 * This file contains the configuration for API endpoints and environment-specific settings.
 */

// API Base URL - can be configured via environment variables
export const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL ||
  "http://localhost:4092/api/v1";

// API Endpoints
export const API_ENDPOINTS = {
  // Health & Status
  health: "/health",
  status: "/status",

  // Room Management
  rooms: {
    base: (name: string) => `/rooms/${encodeURIComponent(name)}`,
    permissions: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/permissions`,
    settings: (name: string) => `/rooms/${encodeURIComponent(name)}/settings`,
    tokens: (name: string) => `/rooms/${encodeURIComponent(name)}/tokens`,
    validateToken: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/tokens/validate`,
    revokeToken: (name: string, jti: string) =>
      `/rooms/${encodeURIComponent(name)}/tokens/${jti}`,
  },

  // Content Management
  content: {
    base: (name: string) => `/rooms/${encodeURIComponent(name)}/contents`,
    prepare: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/contents/prepare`,
    byId: (name: string, contentId: string) =>
      `/rooms/${encodeURIComponent(name)}/contents/${contentId}`,
  },

  // Chunked Upload
  chunkedUpload: {
    prepare: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/uploads/chunks/prepare`,
    upload: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/uploads/chunks`,
    status: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/uploads/chunks/status`,
    complete: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/uploads/chunks/complete`,
  },

  // Authentication
  auth: {
    refresh: "/auth/refresh",
    logout: "/auth/logout",
    cleanup: "/auth/cleanup",
  },
} as const;

// Request Configuration
export const REQUEST_CONFIG = {
  timeout: 30000, // 30 seconds
  retries: 3,
  retryDelay: 1000, // 1 second
} as const;

// Token Configuration
export const TOKEN_CONFIG = {
  storageKey: "elizabeth_tokens",
  refreshBeforeExpiry: 5 * 60 * 1000, // Refresh 5 minutes before expiry
} as const;
