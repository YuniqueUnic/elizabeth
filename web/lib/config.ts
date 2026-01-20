/**
 * API Configuration
 *
 * This file contains the configuration for API endpoints and environment-specific settings.
 */

const DEFAULT_API_PATH = "/api/v1";

const normalizePath = (
  value: string | undefined,
  fallback: string = DEFAULT_API_PATH,
): string => {
  const raw = (value ?? fallback).trim();

  if (!raw || raw === "/") {
    return "";
  }

  const withLeadingSlash = raw.startsWith("/") ? raw : `/${raw}`;
  return withLeadingSlash.replace(/\/+$/, "");
};

const parseBase = (
  value: string | undefined,
  fallbackPath: string = DEFAULT_API_PATH,
) => {
  if (!value) {
    return {
      origin: "",
      path: normalizePath(fallbackPath),
    };
  }

  try {
    const url = new URL(value);
    return {
      origin: url.origin.replace(/\/+$/, ""),
      path: normalizePath(
        url.pathname && url.pathname !== "/" ? url.pathname : fallbackPath,
        fallbackPath,
      ),
    };
  } catch {
    return {
      origin: "",
      path: normalizePath(value, fallbackPath),
    };
  }
};
const parseOrigin = (value: string | undefined): string => {
  if (!value) return "";
  try {
    return new URL(value).origin.replace(/\/+$/, "");
  } catch {
    return "";
  }
};

const publicBase = parseBase(process.env.NEXT_PUBLIC_API_URL, DEFAULT_API_PATH);
const internalBase = parseBase(
  process.env.INTERNAL_API_URL,
  publicBase.path || DEFAULT_API_PATH,
);
const appOrigin = parseOrigin(process.env.NEXT_PUBLIC_APP_URL);

// API Base URL exposed to the browser (path only, no origin)
export const API_BASE_PATH = publicBase.path;
export const API_BASE_URL = API_BASE_PATH;
export const API_BASE_ORIGIN = publicBase.origin;

// Internal API base used by the Next.js server to reach the backend
export const INTERNAL_API_PATH = internalBase.path;
export const INTERNAL_API_ORIGIN = internalBase.origin;
export const INTERNAL_API_BASE_URL = INTERNAL_API_ORIGIN
  ? `${INTERNAL_API_ORIGIN}${INTERNAL_API_PATH}`
  : "";

// Public app origin (useful as fallback when building server-side URLs)
export const PUBLIC_APP_ORIGIN = appOrigin;
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
    messages: (name: string) =>
      `/rooms/${encodeURIComponent(name)}/messages`,
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

export const REQUEST_CONFIG = {
  timeout: 30000, // 30 seconds
  retries: 3,
  retryDelay: 1000, // 1 second
} as const;

export const TOKEN_CONFIG = {
  storageKey: "elizabeth_tokens",
  refreshBeforeExpiry: 5 * 60 * 1000, // Refresh 5 minutes before expiry
} as const;
