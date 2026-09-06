"use client";

import { useCallback, useMemo } from "react";
import { usePathname } from "next/navigation";
import { getRoomToken, getRoomTokenString } from "@/lib/utils/api";
import { useAppStore } from "@/lib/store";
import { decodeJWT } from "@/lib/utils/jwt";
import type { Capability, Grant, RoleDefinition } from "@/lib/types";

const capabilityNames = [
  "room.share", "room.settings.update", "room.roles.manage", "room.delete",
  "msg.read", "msg.send", "msg.copy", "msg.edit", "msg.delete",
  "file.list", "file.preview", "file.download", "file.upload", "file.delete", "file.policy.manage",
  "msg.visibility.manage", "file.visibility.manage",
] as const satisfies readonly Capability[];

export function useRoomCapabilities(
  roles?: RoleDefinition[] | null,
  capabilities?: Grant[] | null,
) {
  const pathname = usePathname();
  const roomName = pathname?.split("/").filter(Boolean)[0] ?? undefined;
  // 角色矩阵变更后 bump，强制从 localStorage 重读最新能力快照
  const capabilitiesVersion = useAppStore((state) => state.capabilitiesVersion);
  const token = useMemo(
    () => {
      void capabilitiesVersion; // 版本号仅作为重读 localStorage 的触发器
      return roomName ? getRoomTokenString(roomName) : null;
    },
    [roomName, capabilitiesVersion],
  );
  const tokenInfo = useMemo(
    () => {
      void capabilitiesVersion;
      return roomName ? getRoomToken(roomName) : null;
    },
    [roomName, capabilitiesVersion],
  );
  const payload = useMemo(() => token ? decodeJWT(token) : null, [token]);
  const effectiveGrants = useMemo(() => {
    const grants = capabilities ?? tokenInfo?.capabilities ?? [];
    if (grants.length > 0) return grants;
    const role = roles?.find((item) => item.role_key === (tokenInfo?.roleKey ?? payload?.role));
    return role?.capabilities ?? [];
  }, [capabilities, tokenInfo, roles, payload?.role]);
  const has = useCallback(
    (capability: Capability, scope: Grant["scope"] = "any") =>
      effectiveGrants.some((grant) => grant.capability === capability && (scope === "any" || grant.scope === scope)),
    [effectiveGrants],
  );
  const can = useMemo(() => ({
    read: has("msg.read"), edit: has("msg.edit", "any") || has("msg.edit", "own"),
    share: has("room.share"), delete: has("room.delete") || has("msg.delete", "any"),
    settings: has("room.settings.update"), manageRoles: has("room.roles.manage"),
    upload: has("file.upload"), download: has("file.download"),
  }), [has]);
  return { token, payload, grants: effectiveGrants, capabilities: effectiveGrants.map((grant) => grant.capability), has, can, roleKey: payload?.role ?? tokenInfo?.roleKey ?? null, roomName: payload?.room_name ?? roomName ?? null, roomId: payload?.room_id ?? null };
}

export { capabilityNames };
