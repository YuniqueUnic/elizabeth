"use client";

import { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { RoomDetails, UploadFileTypeMode, UploadFileTypePolicy } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { getPublicConfig } from "@/api/publicConfigService";
import { updateRoomSettings } from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
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

const UPLOAD_FILE_TYPE_MODES: UploadFileTypeMode[] = ["any", "allow", "deny"];

/** 与后端 normalize_upload_file_extensions 一致的本地预清理；服务端仍是权威校验。 */
function parseExtensionsInput(raw: string): string[] {
  const normalized = raw
    .split(/[,，\s]+/)
    .map((entry) => entry.trim().replace(/^\.+/, "").toLowerCase())
    .filter((entry) => entry.length > 0);
  return [...new Set(normalized)];
}

/** 后端 naive datetime 为 UTC；解析失败时回退原始字符串。 */
function formatExpiry(value: string): string {
  const normalized = value.replace(/(\.\d{3})\d+$/, "$1");
  const date = new Date(value.includes("T") ? `${normalized}Z` : normalized);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/** 房间设置（仅拥有 room.settings.update 的身份可见）：密码、最大进入次数、上传文件类型。 */
export function RoomConfigForm({ roomDetails }: { roomDetails: RoomDetails }) {
  const t = useTranslations("room.config");
  const roomName = useAppStore((state) => state.currentRoomId);
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { can } = useRoomCapabilities();
  const [password, setPassword] = useState("");
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);
  const [removePassword, setRemovePassword] = useState(false);
  const [fileTypeMode, setFileTypeMode] = useState<UploadFileTypeMode>(
    roomDetails.uploadFileType.mode,
  );
  const [fileTypeExtensions, setFileTypeExtensions] = useState(
    roomDetails.uploadFileType.extensions.join(", "),
  );
  const config = useQuery({ queryKey: ["public-config"], queryFn: getPublicConfig, staleTime: Infinity });
  useEffect(() => setMaxViews(roomDetails.settings.maxViews), [roomDetails.settings.maxViews]);
  useEffect(() => {
    setFileTypeMode(roomDetails.uploadFileType.mode);
    setFileTypeExtensions(roomDetails.uploadFileType.extensions.join(", "));
  }, [roomDetails.uploadFileType]);
  const mutation = useMutation({
    mutationFn: () => {
      const uploadFileType: UploadFileTypePolicy = {
        mode: fileTypeMode,
        extensions: fileTypeMode === "any" ? [] : parseExtensionsInput(fileTypeExtensions),
      };
      return updateRoomSettings(roomName, {
        password: password || undefined,
        removePassword,
        maxViews,
        uploadFileType,
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
        description: describeFileTypePolicyError(message, t) || undefined,
        variant: "destructive",
      });
    },
  });
  const expiry = config.data?.room.expiry;
  return (
    <section className="space-y-3">
      <h3 className="text-sm font-semibold">{t("title")}</h3>
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
        <Label htmlFor="upload-file-type-mode">{t("uploadFileType.label")}</Label>
        <Select
          value={fileTypeMode}
          onValueChange={(value) => setFileTypeMode(value as UploadFileTypeMode)}
        >
          <SelectTrigger id="upload-file-type-mode" data-testid="upload-file-type-mode">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {UPLOAD_FILE_TYPE_MODES.map((mode) => (
              <SelectItem key={mode} value={mode}>
                {t(`uploadFileType.mode.${mode}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {fileTypeMode !== "any" && (
          <div className="space-y-1">
            <Label htmlFor="upload-file-type-extensions">
              {t("uploadFileType.extensionsLabel")}
            </Label>
            <Input
              id="upload-file-type-extensions"
              data-testid="upload-file-type-extensions"
              value={fileTypeExtensions}
              onChange={(e) => setFileTypeExtensions(e.target.value)}
              placeholder={t("uploadFileType.extensionsPlaceholder")}
              className="font-mono text-xs"
            />
            <p className="text-xs text-muted-foreground">{t("uploadFileType.extensionsHint")}</p>
          </div>
        )}
      </div>
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
      {expiry && (
        <p className="text-xs text-muted-foreground">
          {roomDetails.settings.expiresAt
            ? formatExpiry(roomDetails.settings.expiresAt)
            : t("expiry.placeholder")}
        </p>
      )}
    </section>
  );
}

/** 后端策略校验消息 → 本地化文案；未匹配时回退原始消息。 */
type Translate = (key: string, values?: Record<string, string | number>) => string;

function describeFileTypePolicyError(message: string, t: Translate): string {
  const stripped = message.startsWith("Validation error: ")
    ? message.slice("Validation error: ".length)
    : message;
  const m = stripped;
  if (
    m ===
    "upload_file_type.extensions must not be empty for allow or deny mode"
  ) {
    return t("uploadFileType.errors.empty");
  }
  if (m.startsWith("upload_file_type.extensions supports at most")) {
    return t("uploadFileType.errors.tooMany");
  }
  if (m.startsWith("Invalid file type extension: ")) {
    return t("uploadFileType.errors.invalid", {
      extension: m.slice("Invalid file type extension: ".length),
    });
  }
  return message;
}
