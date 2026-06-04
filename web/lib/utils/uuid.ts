/**
 * UUID generation utility that works in all contexts.
 *
 * crypto.randomUUID() is only available in secure contexts (HTTPS/localhost).
 * This module provides a fallback for non-HTTPS environments.
 */

function randomHexByte(): string {
  const array = new Uint8Array(1);
  crypto.getRandomValues(array);
  return array[0].toString(16).padStart(2, "0");
}

function generateUUIDv4Fallback(): string {
  // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
  // where y is 8, 9, a, or b
  const hex = Array.from({ length: 32 }, () => randomHexByte()).join("");
  return (
    hex.slice(0, 8) +
    "-" +
    hex.slice(8, 12) +
    "-4" +
    hex.slice(13, 16) +
    "-" +
    ((parseInt(hex.slice(16, 17), 16) & 0x3) | 0x8).toString(16) +
    hex.slice(17, 20) +
    "-" +
    hex.slice(20, 32)
  );
}

/**
 * Generate a UUID v4 string.
 * Uses crypto.randomUUID() when available (secure context),
 * falls back to a manual implementation otherwise.
 */
export function generateUUID(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return generateUUIDv4Fallback();
}
