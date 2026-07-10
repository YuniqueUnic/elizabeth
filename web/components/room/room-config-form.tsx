"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Eye, EyeOff } from "lucide-react";

import type { RoomDetails, RoomPermission } from "@/lib/types";
import { encodePermissions, parsePermissions } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { clearRoomToken } from "@/lib/utils/api";
import { getAccessToken } from "@/api/authService";
import { updateRoomPermissions, updateRoomSettings } from "@/api/roomService";
import { getPublicConfig } from "@/api/publicConfigService";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { isPermissionDeniedError } from "@/lib/utils/mutations";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";

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
import { useLocale, useTranslations } from "next-intl";

interface RoomConfigFormProps {
  roomDetails: RoomDetails;
}

const EXPIRY_UNITS = [
  { seconds: 365 * 24 * 60 * 60, unit: "year" },
  { seconds: 24 * 60 * 60, unit: "day" },
  { seconds: 60 * 60, unit: "hour" },
  { seconds: 60, unit: "minute" },
  { seconds: 1, unit: "second" },
] as const;

function getExpiryOptionFromDate(
  expiresAt: string | null | undefined,
  allowedAges: number[],
  defaultAge: number,
): number {
  if (!expiresAt || allowedAges.length === 0) return defaultAge;

  const expiresAtUTC = expiresAt.endsWith("Z") ? expiresAt : `${expiresAt}Z`;
  const expireTime = new Date(expiresAtUTC).getTime();
  const now = Date.now();
  const diffSeconds = Math.max(0, Math.round((expireTime - now) / 1000));

  // 如果已过期或即将过期，默认 1 分钟
  if (diffSeconds <= 0) return allowedAges[0] ?? defaultAge;

  let closestOption = allowedAges[0] ?? defaultAge;
  let minDiff = Math.abs(diffSeconds - closestOption);

  for (const option of allowedAges) {
    const currentDiff = Math.abs(diffSeconds - option);
    if (currentDiff < minDiff) {
      minDiff = currentDiff;
      closestOption = option;
    }
  }

  return closestOption;
}

function formatExpiryAge(ageSeconds: number, locale: string): string {
  const selectedUnit = EXPIRY_UNITS.find(
    ({ seconds }) => ageSeconds >= seconds && ageSeconds % seconds === 0,
  ) ?? EXPIRY_UNITS[EXPIRY_UNITS.length - 1];
  const value = ageSeconds / selectedUnit.seconds;

  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: selectedUnit.unit,
    unitDisplay: "long",
  }).format(value);
}

// 权限位定义
const PERMISSIONS = {
  VIEW_ONLY: 1, // 0001 - 预览权限
  EDITABLE: 1 << 1, // 0010 - 编辑权限
  SHARE: 1 << 2, // 0100 - 分享权限
  DELETE: 1 << 3, // 1000 - 删除权限
} as const;

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
  const t = useTranslations("room");
  const locale = useLocale();
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const setRoomRedirectTarget = useAppStore((state) =>
    state.setRoomRedirectTarget
  );
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { can } = useRoomPermissions(roomDetails.permissions);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const publicConfigQuery = useQuery({
    queryKey: ["public-config"],
    queryFn: getPublicConfig,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const expiryPolicy = publicConfigQuery.data?.room.expiry;
  const allowedAges = useMemo(
    () => expiryPolicy?.allowed_ages_seconds ?? [],
    [expiryPolicy],
  );
  const defaultAge = expiryPolicy?.default_age_seconds ?? 0;
  const baseExpiryOption = useMemo(
    () => expiryPolicy
      ? getExpiryOptionFromDate(
        roomDetails.settings.expiresAt,
        allowedAges,
        defaultAge,
      )
      : null,
    [allowedAges, defaultAge, expiryPolicy, roomDetails.settings.expiresAt],
  );

  const [expiryOption, setExpiryOption] = useState<number | null>(null);
  const [password, setPassword] = useState(roomDetails.password || "");
  const [showPassword, setShowPassword] = useState(false);
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);
  const [permissionFlags, setPermissionFlags] = useState(() =>
    permissionsToFlags(roomDetails.permissions)
  );

  useEffect(() => {
    setExpiryOption(baseExpiryOption);
    setPassword(roomDetails.password || "");
    setMaxViews(roomDetails.settings.maxViews);
    setPermissionFlags(permissionsToFlags(roomDetails.permissions));
  }, [baseExpiryOption, roomDetails]);

  const canModify = can.delete;

  const basePassword = roomDetails.password || "";
  const baseMaxViews = roomDetails.settings.maxViews;
  const basePermissionFlags = useMemo(
    () => permissionsToFlags(roomDetails.permissions),
    [roomDetails.permissions],
  );

  const settingsChanged = useMemo(() => {
    return (
      (baseExpiryOption !== null && expiryOption !== baseExpiryOption) ||
      password.trim() !== basePassword ||
      maxViews !== baseMaxViews
    );
  }, [expiryOption, baseExpiryOption, password, basePassword, maxViews, baseMaxViews]);

  const permissionsChanged = permissionFlags !== basePermissionFlags;
  const hasAnyChanges = settingsChanged || permissionsChanged;

  const allPermissions: RoomPermission[] = ["read", "edit", "share", "delete"];

  const permissionLabels: Record<RoomPermission, string> = {
    read: t("config.permissions.labels.read"),
    edit: t("config.permissions.labels.edit"),
    share: t("config.permissions.labels.share"),
    delete: t("config.permissions.labels.delete"),
  };

  const permissionDescriptions: Record<RoomPermission, string> = {
    read: t("config.permissions.descriptions.read"),
    edit: t("config.permissions.descriptions.edit"),
    share: t("config.permissions.descriptions.share"),
    delete: t("config.permissions.descriptions.delete"),
  };

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
    const value = `${window.location.origin}/${slug}`;
    try {
      await copyTextToClipboard(value);
      toast({ title: t("config.linkCopied") });
    } catch (error) {
      console.error("Failed to copy redirect url:", error);
      setManualCopyValue(value);
      toast({
        title: t("config.copyFailTitle"),
        description: t("config.copyFailDescription"),
        variant: "destructive",
      });
    }
  };

  const saveMutation = useMutation({
    mutationFn: async () => {
      const oldIdentifier = currentRoomId;
      const oldPermissionValue = encodePermissions(roomDetails.permissions);

      const newPassword = password.trim();
      const passwordChanged = newPassword !== basePassword;

      const settingsPayload: Parameters<typeof updateRoomSettings>[1] = {};
      if (
        expiryOption !== null &&
        baseExpiryOption !== null &&
        expiryOption !== baseExpiryOption
      ) {
        settingsPayload.ageSeconds = expiryOption;
      }
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

      toast({ title: t("config.save.successTitle"), description: t("config.save.successDescription") });

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
      const permissionDenied = isPermissionDeniedError(error);
      toast({
        title: permissionDenied
          ? t("permissionDenied.title")
          : t("config.save.failTitle"),
        description: permissionDenied
          ? t("permissionDenied.roomConfig")
          : error?.message || t("config.save.failDescription"),
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
    <>
      <div className="space-y-4">
      <div>
        <h3 className="mb-3 text-sm font-semibold">{t("config.title")}</h3>

        {!canModify && (
          <p className="text-xs text-muted-foreground mb-3 p-2 bg-muted rounded-md">
            {t("config.adminOnly")}
          </p>
        )}

        <div className="space-y-2 mt-2">
          <Label htmlFor="expires-at">{t("config.expiry.label")}</Label>
          <Select
            value={expiryOption?.toString()}
            onValueChange={(value) => setExpiryOption(Number(value))}
            disabled={
              !canModify ||
              publicConfigQuery.isPending ||
              publicConfigQuery.isError ||
              allowedAges.length === 0
            }
          >
            <SelectTrigger className="w-full" aria-busy={publicConfigQuery.isPending}>
              <SelectValue placeholder={t("config.expiry.placeholder")} />
            </SelectTrigger>
            <SelectContent>
              {allowedAges.map((ageSeconds) => (
                <SelectItem key={ageSeconds} value={ageSeconds.toString()}>
                  {formatExpiryAge(ageSeconds, locale)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {publicConfigQuery.isError && (
            <p role="alert" className="text-xs text-destructive">
              {t("config.save.failDescription")}
            </p>
          )}
        </div>

        <div className="space-y-2 mt-2">
          <Label htmlFor="password">{t("config.password.label")}</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t("config.password.placeholder")}
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
          <Label htmlFor="max-views">{t("config.maxViews.label")}</Label>
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
          <h3 className="text-sm font-semibold">{t("config.permissions.title")}</h3>
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
            {t("config.permissions.hint")}
          </p>
        </div>

        {canModify && (
          <div className="flex gap-2 mt-4">
            <Button
              onClick={() => saveMutation.mutate()}
              className="flex-1"
              disabled={!hasAnyChanges || saveMutation.isPending}
            >
              {saveMutation.isPending ? t("config.save.saving") : t("config.save.saveConfig")}
            </Button>
            {hasAnyChanges && (
              <Button variant="outline" onClick={resetAll}>
                {t("config.cancel")}
              </Button>
            )}
          </div>
        )}

        {!canModify && (
          <p className="text-xs text-muted-foreground mt-2">
            {t("config.adminSaveOnly")}
          </p>
        )}
      </div>
      </div>
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setManualCopyValue("");
        }}
      />
    </>
  );
}
