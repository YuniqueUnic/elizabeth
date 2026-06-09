/**
 * React hook for checking room permissions from JWT token and latest room state.
 */

import { useMemo } from "react";
import { usePathname } from "next/navigation";
import { getRoomTokenString } from "@/lib/utils/api";
import {
    decodeJWT,
    getPermissionsFromToken,
    type JWTPayload,
} from "@/lib/utils/jwt";
import type { RoomPermission } from "@/lib/types";

function resolveEffectivePermissions(
    tokenPermissions: RoomPermission[],
    roomPermissions?: RoomPermission[] | null,
): RoomPermission[] {
    if (!roomPermissions) {
        return tokenPermissions;
    }

    const latestRoomPermissions = new Set(roomPermissions);
    return tokenPermissions.filter((permission) =>
        latestRoomPermissions.has(permission)
    );
}

/**
 * Hook to get and check room permissions.
 *
 * `roomPermissions` should be the latest permissions returned by the room
 * query. When present, UI actions use token claims AND the latest room state,
 * so users already in the room do not keep stale write controls after a remote
 * permission downgrade.
 */
export function useRoomPermissions(roomPermissions?: RoomPermission[] | null) {
    const pathname = usePathname();
    // 从真实 URL 解析房间名，避免静态导出时 useParams() 返回编译期占位符
    const roomName = pathname.split("/").filter(Boolean)[0] ?? undefined;

    const token = useMemo(() => {
        if (!roomName) return null;
        return getRoomTokenString(roomName);
    }, [roomName]);

    const permissions = useMemo(() => {
        if (!token) return [];
        return getPermissionsFromToken(token);
    }, [token]);

    const payload = useMemo<JWTPayload | null>(() => {
        if (!token) return null;
        return decodeJWT(token);
    }, [token]);

    const effectivePermissions = useMemo(
        () => resolveEffectivePermissions(permissions, roomPermissions),
        [permissions, roomPermissions],
    );

    const can = useMemo(
        () => ({
            read: effectivePermissions.includes("read"),
            edit: effectivePermissions.includes("edit"),
            share: effectivePermissions.includes("share"),
            delete: effectivePermissions.includes("delete"),
        }),
        [effectivePermissions],
    );

    const hasAny = useMemo(
        () => (perms: RoomPermission[]) =>
            perms.some((permission) => effectivePermissions.includes(permission)),
        [effectivePermissions],
    );

    const hasAll = useMemo(
        () => (perms: RoomPermission[]) =>
            perms.every((permission) => effectivePermissions.includes(permission)),
        [effectivePermissions],
    );

    return {
        token,
        permissions: effectivePermissions,
        tokenPermissions: permissions,
        roomPermissions,
        payload,
        can,
        hasAny,
        hasAll,
        roomName: payload?.room_name ?? roomName ?? null,
        roomId: payload?.room_id ?? null,
    };
}
