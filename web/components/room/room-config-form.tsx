"use client";

import { useEffect, useState } from "react";
import { useLocale, useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { RoomDetails, UploadFileTypeMode } from "@/lib/types";
import {
  formatBackendDateTime,
  formatDuration,
  formatFileSize,
  parseBackendDateTime,
} from "@/lib/utils/format";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { getPublicConfig } from "@/api/publicConfigService";
import { listRoomRoles, updateRoomSettings } from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import {
  UploadFileTypePolicyFields,
  buildUploadFileTypePolicy,
  describeUploadFileTypeError,
} from "@/components/room/upload-file-type-policy-fields";
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

/**
 * 房间只持久化绝对过期时刻，不记录建房时选定的时长。
 * 因此以「剩余时间最接近的允许时长」作为当前选择；用户未改动时不会回写，
 * 显示上的近似不会漂移到真实过期时刻。
 */
function currentDurationOption(
  expiresAt: string | null,
  allowedAges: number[],
  defaultAge: number,
): number | null {
  if (allowedAges.length === 0) return null;
  if (!expiresAt) return defaultAge;
  const expiresAtDate = parseBackendDateTime(expiresAt);
  if (!expiresAtDate) return defaultAge;
  const remainingSeconds = Math.round((expiresAtDate.getTime() - Date.now()) / 1000);
  if (remainingSeconds <= 0) return allowedAges[0];
  return allowedAges.reduce((closest, candidate) =>
    Math.abs(candidate - remainingSeconds) < Math.abs(closest - remainingSeconds)
      ? candidate
      : closest,
  );
}

/** 房间设置（仅拥有 room.settings.update 的身份可见）：持续时间、密码、容量、进入次数、默认角色、上传文件类型。 */
export function RoomConfigForm({ roomDetails }: { roomDetails: RoomDetails }) {
  const t = useTranslations("room.config");
  const locale = useLocale();
  const roomName = useAppStore((state) => state.currentRoomId);
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { token, can } = useRoomCapabilities();
  const [password, setPassword] = useState("");
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);
  const [maxSize, setMaxSize] = useState(String(roomDetails.maxSize));
  const [defaultRole, setDefaultRole] = useState(roomDetails.defaultRoleKey);
  const [removePassword, setRemovePassword] = useState(false);
  const [duration, setDuration] = useState<number | null>(null);
  const [fileTypeMode, setFileTypeMode] = useState<UploadFileTypeMode>(
    roomDetails.uploadFileType.mode,
  );
  const [fileTypeExtensions, setFileTypeExtensions] = useState(
    roomDetails.uploadFileType.extensions.join(", "),
  );
  const config = useQuery({ queryKey: ["public-config"], queryFn: getPublicConfig, staleTime: Infinity });
  // 默认角色必须是本房角色集成员，因此选项来自房间角色矩阵（与权限对话框共用缓存）。
  const rolesQuery = useQuery({
    queryKey: ["room-roles", roomName],
    queryFn: () => listRoomRoles(roomName, token ?? undefined),
    enabled: Boolean(token),
    staleTime: 15_000,
  });
  const roles = rolesQuery.data ?? [];
  const expiryPolicy = config.data?.room.expiry;
  const allowedAges = expiryPolicy?.allowed_ages_seconds ?? [];
  const currentDuration = expiryPolicy
    ? currentDurationOption(
        roomDetails.settings.expiresAt,
        allowedAges,
        expiryPolicy.default_age_seconds,
      )
    : null;
  const durationChanged =
    duration !== null && currentDuration !== null && duration !== currentDuration;
  useEffect(() => setDuration(currentDuration), [currentDuration]);
  useEffect(() => {
    setMaxViews(roomDetails.settings.maxViews);
    setMaxSize(String(roomDetails.maxSize));
    setDefaultRole(roomDetails.defaultRoleKey);
  }, [roomDetails.settings.maxViews, roomDetails.maxSize, roomDetails.defaultRoleKey]);
  useEffect(() => {
    setFileTypeMode(roomDetails.uploadFileType.mode);
    setFileTypeExtensions(roomDetails.uploadFileType.extensions.join(", "));
  }, [roomDetails.uploadFileType]);
  const parsedMaxSize = Number(maxSize);
  // 只在确实改动时提交，避免保存密码等操作顺带重写容量/角色。
  const maxSizeChanged =
    Number.isInteger(parsedMaxSize) && parsedMaxSize > 0 && parsedMaxSize !== roomDetails.maxSize;
  const defaultRoleChanged = defaultRole !== roomDetails.defaultRoleKey;
  const mutation = useMutation({
    mutationFn: () => {
      return updateRoomSettings(roomName, {
        password: password || undefined,
        removePassword,
        ageSeconds: durationChanged && duration !== null ? duration : undefined,
        maxViews,
        maxSize: maxSizeChanged ? parsedMaxSize : undefined,
        defaultRoleKey: defaultRoleChanged ? defaultRole : undefined,
        uploadFileType: buildUploadFileTypePolicy(fileTypeMode, fileTypeExtensions),
      });
    },
    onSuccess: (room) => {
      queryClient.setQueryData(["room", roomName], room);
      setPassword("");
      setRemovePassword(false);
      toast({ title: t("save.successTitle") });
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : "";
      toast({
        title: t("save.failTitle"),
        description:
          describeUploadFileTypeError(message, (key, values) =>
            t(`uploadFileType.${key}`, values),
          ) || undefined,
        variant: "destructive",
      });
    },
  });
  return (
    <section className="space-y-3">
      <h3 className="text-sm font-semibold">{t("title")}</h3>
      <div className="space-y-2">
        <Label htmlFor="room-duration">{t("duration.label")}</Label>
        <Select
          value={duration?.toString()}
          onValueChange={(value) => setDuration(Number(value))}
          disabled={!can.settings || allowedAges.length === 0}
        >
          <SelectTrigger id="room-duration" data-testid="room-duration-select" className="w-full">
            <SelectValue placeholder={t("duration.placeholder")} />
          </SelectTrigger>
          <SelectContent>
            {allowedAges.map((ageSeconds) => (
              <SelectItem
                key={ageSeconds}
                value={ageSeconds.toString()}
                data-testid={`room-duration-option-${ageSeconds}`}
              >
                {formatDuration(ageSeconds, locale)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {roomDetails.settings.expiresAt && (
          <p className="text-xs text-muted-foreground" data-testid="room-expiry-hint">
            {t("expiresAt", { time: formatBackendDateTime(roomDetails.settings.expiresAt) })}
          </p>
        )}
      </div>
      <div className="space-y-2">
        <Label htmlFor="room-password">{t("password.label")}</Label>
        <Input
          id="room-password"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="new-password"
          placeholder={t("password.placeholder")}
        />
        {roomDetails.settings.passwordProtected && (
          <p className="text-xs text-muted-foreground">{t("password.protectedHint")}</p>
        )}
      </div>
      <div className="space-y-2">
        <Label htmlFor="room-max-views">{t("maxViews.label")}</Label>
        <Input
          id="room-max-views"
          type="number"
          min={0}
          value={maxViews}
          onChange={(e) => setMaxViews(Number(e.target.value))}
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="room-max-size">{t("maxSize.label")}</Label>
        <Input
          id="room-max-size"
          data-testid="room-max-size"
          type="number"
          min={1}
          inputMode="numeric"
          value={maxSize}
          onChange={(e) => setMaxSize(e.target.value)}
        />
        <p className="text-xs text-muted-foreground" data-testid="room-max-size-hint">
          {t("maxSize.hint", {
            size: formatFileSize(roomDetails.maxSize),
            used: formatFileSize(roomDetails.currentSize),
          })}
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="room-default-role">{t("defaultRole.label")}</Label>
        <Select
          value={defaultRole}
          onValueChange={setDefaultRole}
          disabled={!can.settings || roles.length === 0}
        >
          <SelectTrigger
            id="room-default-role"
            data-testid="room-default-role-select"
            className="w-full"
          >
            <SelectValue placeholder={t("defaultRole.placeholder")} />
          </SelectTrigger>
          <SelectContent>
            {roles.map((role) => (
              <SelectItem
                key={role.role_key}
                value={role.role_key}
                data-testid={`room-default-role-option-${role.role_key}`}
              >
                {role.display_name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">{t("defaultRole.hint")}</p>
      </div>
      <UploadFileTypePolicyFields
        mode={fileTypeMode}
        extensions={fileTypeExtensions}
        onModeChange={setFileTypeMode}
        onExtensionsChange={setFileTypeExtensions}
        disabled={!can.settings}
        testIdPrefix="upload-file-type"
      />
      <div className="flex items-center gap-2">
        <Button type="button" onClick={() => mutation.mutate()} disabled={!can.settings || mutation.isPending}>
          {mutation.isPending ? t("save.saving") : t("save.saveConfig")}
        </Button>
        {roomDetails.settings.passwordProtected && (
          <Button
            type="button"
            variant="ghost"
            onClick={() => setRemovePassword(true)}
            disabled={!can.settings}
          >
            {t("password.removeAction")}
          </Button>
        )}
      </div>
    </section>
  );
}
