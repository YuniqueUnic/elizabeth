/**
 * React hook for checking room permissions from JWT token
 */

import { useMemo } from "react";
import { useParams } from "next/navigation";
import { getRoomTokenString } from "@/lib/utils/api";
import {
    decodeJWT,
    getPermissionsFromToken,
    hasAllPermissions,
    hasAnyPermission,
    hasPermission,
    type JWTPayload,
} from "@/lib/utils/jwt";
import type { RoomPermission } from "@/lib/types";

/**
 * Hook to get and check room permissions
 */
export function useRoomPermissions() {
    const params = useParams();
    const roomName = params.roomName as string | undefined;

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

    const can = useMemo(
        () => ({
            read: hasPermission(token, "read"),
            edit: hasPermission(token, "edit"),
            share: hasPermission(token, "share"),
            delete: hasPermission(token, "delete"),
        }),
        [token],
    );

    const hasAny = useMemo(
        () => (perms: RoomPermission[]) => hasAnyPermission(token, perms),
        [token],
    );

    const hasAll = useMemo(
        () => (perms: RoomPermission[]) => hasAllPermissions(token, perms),
        [token],
    );

    return {
        token,
        permissions,
        payload,
        can,
        hasAny,
        hasAll,
        roomName: payload?.room_name ?? roomName ?? null,
        roomId: payload?.room_id ?? null,
    };
}
