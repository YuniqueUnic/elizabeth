import type { RoomTokenClaims } from "../types";

export type JWTPayload = RoomTokenClaims;

export function decodeJWT(token: string): JWTPayload | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;
    const normalized = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    return JSON.parse(atob(normalized)) as JWTPayload;
  } catch {
    return null;
  }
}

export function getRoomNameFromToken(token: string | null | undefined): string | null {
  return token ? decodeJWT(token)?.room_name ?? null : null;
}
