import { API_BASE_ORIGIN, PUBLIC_APP_ORIGIN } from "../config";

export function resolveWebSocketUrl(path: string = "/api/v1/ws"): string {
  const explicit = process.env.NEXT_PUBLIC_WS_URL;
  if (explicit && explicit.trim()) {
    return explicit.trim();
  }

  if (API_BASE_ORIGIN) {
    return `${API_BASE_ORIGIN}${path}`;
  }

  if (typeof window !== "undefined") {
    // Production deployments (Docker + gateway) should keep the backend private.
    // Always connect to the same-origin WebSocket endpoint and let the gateway proxy it.
    if (process.env.NODE_ENV === "production") {
      return `${window.location.origin}${path}`;
    }

    // Local dev default: frontend (4001) + backend (4092)
    if (window.location.port === "4001") {
      const hostname = window.location.hostname === "0.0.0.0"
        ? "127.0.0.1"
        : window.location.hostname;
      return `${window.location.protocol}//${hostname}:4092${path}`;
    }
    return `${window.location.origin}${path}`;
  }

  if (PUBLIC_APP_ORIGIN) {
    return `${PUBLIC_APP_ORIGIN}${path}`;
  }

  return `http://127.0.0.1:4092${path}`;
}
