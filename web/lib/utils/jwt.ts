/**
 * JWT Utilities for parsing token claims and permissions
 */

import type { RoomPermission } from "../types";
import { parsePermissions } from "../types";

/**
 * JWT Token Claims structure (matches backend RoomTokenClaims)
 */
export interface JWTPayload {
    sub: string; // "room:{room_id}"
    room_id: number;
    room_name: string;
    permission: number; // Bit flags: 1=read, 2=edit, 4=share, 8=delete
    max_size: number;
    exp: number; // Expiration timestamp
    iat: number; // Issued at timestamp
    jti: string; // JWT ID
    token_type?: "access" | "refresh";
    refresh_jti?: string;
}

/**
 * Decode JWT token payload (without verification)
 * Note: This only decodes, it does not verify the signature
 */
export function decodeJWT(token: string): JWTPayload | null {
    try {
        const parts = token.split(".");
        if (parts.length !== 3) {
            return null;
        }

        const payload = parts[1];
        const decoded = atob(payload.replace(/-/g, "+").replace(/_/g, "/"));
        return JSON.parse(decoded) as JWTPayload;
    } catch (error) {
        console.error("Failed to decode JWT:", error);
        return null;
    }
}

/**
 * Get permissions from JWT token
 */
export function getPermissionsFromToken(token: string): RoomPermission[] {
    const payload = decodeJWT(token);
    if (!payload) {
        return [];
    }

    return parsePermissions(payload.permission);
}

/**
 * Check if token has specific permission
 */
export function hasPermission(
    token: string | null | undefined,
    permission: RoomPermission,
): boolean {
    if (!token) {
        return false;
    }

    const perms = getPermissionsFromToken(token);
    return perms.includes(permission);
}

/**
 * Check if token has any of the specified permissions
 */
export function hasAnyPermission(
    token: string | null | undefined,
    permissions: RoomPermission[],
): boolean {
    if (!token || permissions.length === 0) {
        return false;
    }

    const perms = getPermissionsFromToken(token);
    return permissions.some((p) => perms.includes(p));
}

/**
 * Check if token has all of the specified permissions
 */
export function hasAllPermissions(
    token: string | null | undefined,
    permissions: RoomPermission[],
): boolean {
    if (!token || permissions.length === 0) {
        return false;
    }

    const perms = getPermissionsFromToken(token);
    return permissions.every((p) => perms.includes(p));
}

/**
 * Get room name from JWT token
 */
export function getRoomNameFromToken(
    token: string | null | undefined,
): string | null {
    if (!token) {
        return null;
    }

    const payload = decodeJWT(token);
    return payload?.room_name ?? null;
}
