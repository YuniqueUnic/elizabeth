"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Eye, EyeOff } from "lucide-react";

import type { RoomDetails, RoomPermission } from "@/lib/types";
import { encodePermissions, parsePermissions } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { clearRoomToken } from "@/lib/utils/api";
import { getAccessToken } from "@/api/authService";
import { updateRoomPermissions, updateRoomSettings } from "@/api/roomService";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface RoomConfigFormProps {
  roomDetails: RoomDetails;
}

const EXPIRY_OPTIONS = [
  { label: "1 分钟", value: "1min", ms: 60 * 1000 },
  { label: "10 分钟", value: "10min", ms: 10 * 60 * 1000 },
  { label: "1 小时", value: "1hr", ms: 60 * 60 * 1000 },
  { label: "12 小时", value: "12hr", ms: 12 * 60 * 60 * 1000 },
  { label: "1 天", value: "1day", ms: 24 * 60 * 60 * 1000 },
  { label: "1 周", value: "1week", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "永不过期", value: "never", ms: 0 },
];

function getExpiryOptionFromDate(expiresAt: string | null | undefined): string {
  if (!expiresAt) return "never";

  const expiresAtUTC = expiresAt.endsWith("Z") ? expiresAt : `${expiresAt}Z`;
  const expireTime = new Date(expiresAtUTC).getTime();
  const now = Date.now();
  const diff = expireTime - now;

  if (diff <= 0) return "1min";

  let closestOption = EXPIRY_OPTIONS[0];
  let minDiff = Math.abs(diff - closestOption.ms);

  for (const option of EXPIRY_OPTIONS) {
    if (option.ms === 0) continue;
    const currentDiff = Math.abs(diff - option.ms);
    if (currentDiff < minDiff) {
      minDiff = currentDiff;
      closestOption = option;
    }
  }

  return closestOption.value;
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

function permissionsToFlags(permissions: RoomPermission[]): number {
  let flags = 0;
  if (permissions.includes("read")) flags |= PERMISSIONS.VIEW_ONLY;
  if (permissions.includes("edit")) flags |= PERMISSIONS.EDITABLE;
  if (permissions.includes("share")) flags |= PERMISSIONS.SHARE;
  if (permissions.includes("delete")) flags |= PERMISSIONS.DELETE;
  return flags;
}

function canTogglePermission(
  permission: RoomPermission,
  currentFlags: number,
  newValue: boolean,
): boolean {
  if (newValue) {
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
    if (permission === "read") {
      return (currentFlags &
        (PERMISSIONS.EDITABLE | PERMISSIONS.SHARE | PERMISSIONS.DELETE)) === 0;
    }
    if (permission === "edit") {
      return (currentFlags & PERMISSIONS.DELETE) === 0;
    }
  }
  return true;
}

export function RoomConfigForm({ roomDetails }: RoomConfigFormProps) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const setRoomRedirectTarget = useAppStore((state) =>
    state.setRoomRedirectTarget
  );
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { can } = useRoomPermissions();

  const [expiryOption, setExpiryOption] = useState(() =>
    getExpiryOptionFromDate(roomDetails.settings.expiresAt)
  );
  const [password, setPassword] = useState(roomDetails.password || "");
  const [showPassword, setShowPassword] = useState(false);
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);
  const [permissionFlags, setPermissionFlags] = useState(() =>
    permissionsToFlags(roomDetails.permissions)
  );

  useEffect(() => {
    setExpiryOption(getExpiryOptionFromDate(roomDetails.settings.expiresAt));
    setPassword(roomDetails.password || "");
    setMaxViews(roomDetails.settings.maxViews);
    setPermissionFlags(permissionsToFlags(roomDetails.permissions));
  }, [roomDetails]);

  const canModify = can.delete;

  const baseExpiryOption = useMemo(
    () => getExpiryOptionFromDate(roomDetails.settings.expiresAt),
    [roomDetails.settings.expiresAt],
  );
  const basePassword = roomDetails.password || "";
  const baseMaxViews = roomDetails.settings.maxViews;
  const basePermissionFlags = useMemo(
    () => permissionsToFlags(roomDetails.permissions),
    [roomDetails.permissions],
  );

  const settingsChanged = useMemo(() => {
    return (
      expiryOption !== baseExpiryOption ||
      password.trim() !== basePassword ||
      maxViews !== baseMaxViews
    );
  }, [expiryOption, baseExpiryOption, password, basePassword, maxViews, baseMaxViews]);

  const permissionsChanged = permissionFlags !== basePermissionFlags;
  const hasAnyChanges = settingsChanged || permissionsChanged;

  const allPermissions: RoomPermission[] = ["read", "edit", "share", "delete"];

  const handleTogglePermission = (permission: RoomPermission, checked: boolean) => {
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

  const copyRedirectUrl = async (slug: string) => {
    try {
      await navigator.clipboard.writeText(`${window.location.origin}/${slug}`);
      toast({ title: "已复制新链接" });
    } catch (error) {
      console.error("Failed to copy redirect url:", error);
      toast({
        title: "复制失败",
        description: "无法复制链接，请手动复制。",
        variant: "destructive",
      });
    }
  };

  const saveMutation = useMutation({
    mutationFn: async () => {
      const oldIdentifier = currentRoomId;
      const oldPermissionValue = encodePermissions(roomDetails.permissions);

      // 1) settings patch（仅在发生变化时发字段）
      const option = EXPIRY_OPTIONS.find((opt) => opt.value === expiryOption);
      let expiresAt: string | null | undefined = undefined;
      if (expiryOption !== baseExpiryOption) {
        if (option && option.ms > 0) {
          const now = new Date();
          const expireDate = new Date(now.getTime() + option.ms);
          expiresAt = expireDate.toISOString().replace("Z", "");
        } else {
          // NOTE: backend 当前无法区分“字段缺失”和“null”，此处保守不主动清空 expire_at
          expiresAt = undefined;
        }
      }

      const newPassword = password.trim();
      const passwordChanged = newPassword !== basePassword;

      const settingsPayload: Parameters<typeof updateRoomSettings>[1] = {};
      if (expiresAt !== undefined) settingsPayload.expiresAt = expiresAt;
      if (passwordChanged) settingsPayload.password = newPassword;
      if (maxViews !== baseMaxViews) settingsPayload.maxViews = maxViews;

      let updated: RoomDetails | null = null;
      if (Object.keys(settingsPayload).length > 0) {
        updated = await updateRoomSettings(oldIdentifier, settingsPayload);
        queryClient.setQueryData(["room", oldIdentifier], updated);

        if (passwordChanged) {
          // Refresh token pair so the browser keeps a consistent auth state.
          await getAccessToken(oldIdentifier, newPassword || undefined);
        }
      }

      // 2) permissions update（始终最后执行，避免撤销 delete 后无法改 settings）
      let permissionUpdateResult: RoomDetails | null = null;
      if (permissionsChanged) {
        const perms = parsePermissions(permissionFlags);
        permissionUpdateResult = await updateRoomPermissions(oldIdentifier, perms);
        queryClient.setQueryData(["room", oldIdentifier], permissionUpdateResult);
      }

      const finalRoom = permissionUpdateResult ?? updated ?? roomDetails;
      const newIdentifier = finalRoom.slug || finalRoom.name;
      const newPermissionValue = encodePermissions(parsePermissions(permissionFlags));

      return {
        oldIdentifier,
        newIdentifier,
        finalRoom,
        oldPermissionValue,
        newPermissionValue,
      };
    },
    onSuccess: async (result) => {
      const { oldIdentifier, newIdentifier, finalRoom, oldPermissionValue, newPermissionValue } =
        result;

      toast({ title: "配置已保存", description: "房间配置已成功更新" });

      if (newIdentifier !== oldIdentifier) {
        // slug 变更：保持当前页可用（避免 refetch 触发 401），并提示用户手动跳转
        clearRoomToken(oldIdentifier);
        setRoomRedirectTarget(newIdentifier);
        return;
      }

      // slug 未变：允许正常 refetch
      queryClient.invalidateQueries({ queryKey: ["room", oldIdentifier] });
      queryClient.invalidateQueries({ queryKey: ["contents", oldIdentifier] });
      queryClient.invalidateQueries({ queryKey: ["messages", oldIdentifier] });

      // 权限降级：清 token，强制重新登录，以免 UI/权限状态与后端不一致
      if (newPermissionValue < oldPermissionValue) {
        clearRoomToken(oldIdentifier);
        setTimeout(() => window.location.reload(), 600);
      } else {
        // Ensure local state follows server-returned values on success.
        queryClient.setQueryData(["room", oldIdentifier], finalRoom);
      }
    },
    onError: (error: any) => {
      console.error("Failed to save room config:", error);
      toast({
        title: "保存失败",
        description: error?.message || "无法保存房间配置，请重试",
        variant: "destructive",
      });
    },
  });

  const resetAll = () => {
    setExpiryOption(baseExpiryOption);
    setPassword(basePassword);
    setMaxViews(baseMaxViews);
    setPermissionFlags(basePermissionFlags);
  };

  return (
    <div className="space-y-4">
      <div>
        <h3 className="mb-3 text-sm font-semibold">房间配置</h3>

        {!canModify && (
          <p className="text-xs text-muted-foreground mb-3 p-2 bg-muted rounded-md">
            只有房间管理员（拥有删除权限）可以修改房间配置
          </p>
        )}

        <div className="space-y-2 mt-2">
          <Label htmlFor="expires-at">过期时间</Label>
          <Select
            value={expiryOption}
            onValueChange={setExpiryOption}
            disabled={!canModify}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="选择过期时间" />
            </SelectTrigger>
            <SelectContent>
              {EXPIRY_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2 mt-2">
          <Label htmlFor="password">房间密码</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="设置房间密码"
              disabled={!canModify}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-0 top-0 h-full"
              onClick={() => setShowPassword(!showPassword)}
              disabled={!canModify}
            >
              {showPassword
                ? <EyeOff className="h-4 w-4" />
                : <Eye className="h-4 w-4" />}
            </Button>
          </div>
        </div>

        <div className="space-y-2 mt-2">
          <Label htmlFor="max-views">最大查看次数</Label>
          <Input
            id="max-views"
            type="number"
            value={maxViews}
            onChange={(e) => setMaxViews(Number(e.target.value))}
            min={1}
            disabled={!canModify}
          />
        </div>

        <div className="mt-6 space-y-3">
          <h3 className="text-sm font-semibold">房间权限</h3>
          <div className="flex flex-wrap gap-2">
            {allPermissions.map((permission) => {
              const isEnabled = (permissionFlags &
                (permission === "read"
                  ? PERMISSIONS.VIEW_ONLY
                  : permission === "edit"
                  ? PERMISSIONS.EDITABLE
                  : permission === "share"
                  ? PERMISSIONS.SHARE
                  : PERMISSIONS.DELETE)) !== 0;

              const canToggle = canTogglePermission(
                permission,
                permissionFlags,
                !isEnabled,
              );

              return (
                <button
                  key={permission}
                  onClick={() => handleTogglePermission(permission, !isEnabled)}
                  disabled={!canModify || (!canToggle && !isEnabled)}
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
                    !canModify || (!canToggle && !isEnabled)
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
        </div>

        {canModify && (
          <div className="flex gap-2 mt-4">
            <Button
              onClick={() => saveMutation.mutate()}
              className="flex-1"
              disabled={!hasAnyChanges || saveMutation.isPending}
            >
              {saveMutation.isPending ? "保存中..." : "保存配置"}
            </Button>
            {hasAnyChanges && (
              <Button variant="outline" onClick={resetAll}>
                取消
              </Button>
            )}
          </div>
        )}

        {!canModify && (
          <p className="text-xs text-muted-foreground mt-2">
            只有房间管理员可以保存配置
          </p>
        )}
      </div>
    </div>
  );
}
