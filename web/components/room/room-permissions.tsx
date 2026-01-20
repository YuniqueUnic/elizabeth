"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { updateRoomPermissions } from "@/api/roomService";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { encodePermissions, parsePermissions } from "@/lib/types";
import type { RoomPermission } from "@/lib/types";
import { useRouter } from "next/navigation";
import { clearRoomToken } from "@/lib/utils/api";

interface RoomPermissionsProps {
  permissions: RoomPermission[];
}

// 权限位定义
const PERMISSIONS = {
  VIEW_ONLY: 1, // 0001 - 预览权限
  EDITABLE: 1 << 1, // 0010 - 编辑权限
  SHARE: 1 << 2, // 0100 - 分享权限
  DELETE: 1 << 3, // 1000 - 删除权限
} as const;

const permissionLabels: Record<RoomPermission, string> = {
  read: "预览",
  edit: "编辑",
  share: "分享",
  delete: "删除",
};

const permissionDescriptions: Record<RoomPermission, string> = {
  read: "查看房间内容",
  edit: "上传和修改内容",
  share: "公开分享房间",
  delete: "删除房间内容",
};

// 将 RoomPermission 数组转换为位标志
function permissionsToFlags(permissions: RoomPermission[]): number {
  let flags = 0;
  if (permissions.includes("read")) flags |= PERMISSIONS.VIEW_ONLY;
  if (permissions.includes("edit")) flags |= PERMISSIONS.EDITABLE;
  if (permissions.includes("share")) flags |= PERMISSIONS.SHARE;
  if (permissions.includes("delete")) flags |= PERMISSIONS.DELETE;
  return flags;
}

// 检查权限依赖是否满足
function canTogglePermission(
  permission: RoomPermission,
  currentFlags: number,
  newValue: boolean,
): boolean {
  if (newValue) {
    // 启用权限时，检查依赖
    if (permission === "edit" || permission === "share") {
      return (currentFlags & PERMISSIONS.VIEW_ONLY) !== 0;
    }
    if (permission === "delete") {
      return (
        (currentFlags & PERMISSIONS.VIEW_ONLY) !== 0 &&
        (currentFlags & PERMISSIONS.EDITABLE) !== 0
      );
    }
  } else {
    // 禁用权限时，检查是否有其他权限依赖它
    if (permission === "read") {
      // 如果有编辑、分享或删除权限，不能禁用预览
      return (currentFlags &
        (PERMISSIONS.EDITABLE | PERMISSIONS.SHARE | PERMISSIONS.DELETE)) === 0;
    }
    if (permission === "edit") {
      // 如果有删除权限，不能禁用编辑
      return (currentFlags & PERMISSIONS.DELETE) === 0;
    }
  }
  return true;
}

export function RoomPermissions({ permissions }: RoomPermissionsProps) {
  const router = useRouter();
  const { currentRoomId } = useAppStore();
  const setRoomRedirectTarget = useAppStore((state) =>
    state.setRoomRedirectTarget
  );
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { can } = useRoomPermissions();

  const [permissionFlags, setPermissionFlags] = useState(
    permissionsToFlags(permissions),
  );

  // 当 permissions prop 更新时，同步更新 permissionFlags 状态
  useEffect(() => {
    setPermissionFlags(permissionsToFlags(permissions));
  }, [permissions]);

  const hasChanges = permissionFlags !== permissionsToFlags(permissions);

  const updateMutation = useMutation({
    mutationFn: (newPermissions: RoomPermission[]) =>
      updateRoomPermissions(currentRoomId, newPermissions),
    onSuccess: async (updatedRoom) => {
      const newIdentifier = updatedRoom.slug || updatedRoom.name;
      const oldIdentifier = currentRoomId;

      // 权限更新成功，显示提示
      toast({
        title: "权限已更新",
        description: "房间权限已成功更新",
      });

      // 如果 slug 发生变化，需要跳转到新的 URL
      if (newIdentifier !== oldIdentifier) {
        // 保持当前页面可用（避免立即 refetch 触发错误），同时提示用户手动跳转
        queryClient.setQueryData(["room", oldIdentifier], updatedRoom);
        clearRoomToken(oldIdentifier);
        setRoomRedirectTarget(newIdentifier);
        return;
      } else {
        // 清理旧的查询缓存
        queryClient.invalidateQueries({ queryKey: ["room", oldIdentifier] });
        queryClient.invalidateQueries({ queryKey: ["contents", oldIdentifier] });

        // slug 没有变化，但权限可能降级了
        // 需要清理当前 token，强制用户重新登录以获取新权限的 token
        const oldPermissionValue = encodePermissions(permissions);
        const newPermissionValue = encodePermissions(
          parsePermissions(permissionFlags),
        );

        // 如果权限降级（新权限值小于旧权限值），需要重新登录
        if (newPermissionValue < oldPermissionValue) {
          clearRoomToken(oldIdentifier);

          // 延迟跳转，让用户看到成功提示
          setTimeout(() => {
            toast({
              title: "需要重新登录",
              description: "权限已降级，请重新输入密码登录",
            });

            // 刷新页面，触发重新登录流程
            window.location.reload();
          }, 1500);
        }
      }
    },
    onError: () => {
      toast({
        title: "更新失败",
        description: "无法更新房间权限，请重试",
        variant: "destructive",
      });
    },
  });

  const allPermissions: RoomPermission[] = ["read", "edit", "share", "delete"];

  const handleSave = () => {
    const newPermissions = parsePermissions(permissionFlags);
    updateMutation.mutate(newPermissions);
  };

  const hasPermission = (permission: RoomPermission): boolean => {
    const flag = permission === "read"
      ? PERMISSIONS.VIEW_ONLY
      : permission === "edit"
      ? PERMISSIONS.EDITABLE
      : permission === "share"
      ? PERMISSIONS.SHARE
      : PERMISSIONS.DELETE;
    return (permissionFlags & flag) !== 0;
  };

  const handleToggle = (permission: RoomPermission, checked: boolean) => {
    if (!canTogglePermission(permission, permissionFlags, checked)) {
      return;
    }

    let newFlags = permissionFlags;
    const flag = permission === "read"
      ? PERMISSIONS.VIEW_ONLY
      : permission === "edit"
      ? PERMISSIONS.EDITABLE
      : permission === "share"
      ? PERMISSIONS.SHARE
      : PERMISSIONS.DELETE;

    if (checked) {
      newFlags |= flag;
      // 自动启用依赖的权限
      if (permission === "edit" || permission === "share") {
        newFlags |= PERMISSIONS.VIEW_ONLY;
      }
      if (permission === "delete") {
        newFlags |= PERMISSIONS.VIEW_ONLY | PERMISSIONS.EDITABLE;
      }
    } else {
      newFlags &= ~flag;
    }

    setPermissionFlags(newFlags);
  };

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold">房间权限</h3>

      <div className="flex flex-wrap gap-2">
        {allPermissions.map((permission) => {
          const isEnabled = hasPermission(permission);
          const canToggle = canTogglePermission(
            permission,
            permissionFlags,
            !isEnabled,
          );

          return (
            <button
              key={permission}
              onClick={() => handleToggle(permission, !isEnabled)}
              disabled={!canToggle && !isEnabled}
              aria-pressed={isEnabled}
              data-state={isEnabled ? "on" : "off"}
              className={`
                inline-flex items-center justify-center rounded-full px-4 py-1.5 text-sm font-medium
                transition-all cursor-pointer
                ${
                isEnabled
                  ? "bg-primary text-primary-foreground hover:bg-primary/90"
                  : "bg-secondary text-secondary-foreground hover:bg-secondary/80"
              }
                ${
                !canToggle && !isEnabled
                  ? "opacity-50 cursor-not-allowed"
                  : "shadow-sm"
              }
              `}
              title={`${permissionLabels[permission]}: ${
                permissionDescriptions[permission]
              }`}
            >
              {permissionLabels[permission]}
            </button>
          );
        })}
      </div>
      <p className="text-xs text-muted-foreground italic">
        提示：编辑、分享需要预览权限；删除需要预览和编辑权限
      </p>

      {/* 只有管理员才能修改权限 */}
      {can.delete && (
        <div className="flex gap-2">
          <Button
            onClick={handleSave}
            disabled={!hasChanges || updateMutation.isPending}
            className="flex-1"
            size="sm"
          >
            {updateMutation.isPending ? "保存中..." : "保存权限"}
          </Button>
          {hasChanges && (
            <Button
              variant="outline"
              onClick={() =>
                setPermissionFlags(permissionsToFlags(permissions))}
              size="sm"
            >
              取消
            </Button>
          )}
        </div>
      )}

      {!can.delete && (
        <p className="text-xs text-muted-foreground">
          只有房间管理员可以修改权限
        </p>
      )}
    </div>
  );
}
