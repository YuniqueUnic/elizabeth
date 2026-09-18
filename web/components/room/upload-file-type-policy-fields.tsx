"use client";

import { useTranslations } from "next-intl";
import type { UploadFileTypeMode, UploadFileTypePolicy } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const MODES: UploadFileTypeMode[] = ["any", "allow", "deny"];

/** 与后端 normalize_upload_file_extensions 一致的本地预清理；服务端仍是权威校验。 */
export function parseExtensionsInput(raw: string): string[] {
  const normalized = raw
    .split(/[,，\s]+/)
    .map((entry) => entry.trim().replace(/^\.+/, "").toLowerCase())
    .filter((entry) => entry.length > 0);
  return [...new Set(normalized)];
}

/** 把表单状态折叠成后端策略：any 模式忽略扩展名列表。 */
export function buildUploadFileTypePolicy(
  mode: UploadFileTypeMode,
  extensions: string,
): UploadFileTypePolicy {
  return {
    mode,
    extensions: mode === "any" ? [] : parseExtensionsInput(extensions),
  };
}

type Translate = (key: string, values?: Record<string, string | number>) => string;

/** 后端策略校验消息 → 本地化文案；未匹配时回退原始消息。 */
export function describeUploadFileTypeError(message: string, t: Translate): string {
  const m = message.startsWith("Validation error: ")
    ? message.slice("Validation error: ".length)
    : message;
  if (m === "upload_file_type.extensions must not be empty for allow or deny mode") {
    return t("errors.empty");
  }
  if (m.startsWith("upload_file_type.extensions supports at most")) {
    return t("errors.tooMany");
  }
  if (m.startsWith("Invalid file type extension: ")) {
    return t("errors.invalid", {
      extension: m.slice("Invalid file type extension: ".length),
    });
  }
  return message;
}

/**
 * 房间上传文件类型策略编辑器。房间侧边栏与平台管理面板共用同一实现，
 * 保证两处入口的语义与文案不会再次分叉。
 */
export function UploadFileTypePolicyFields({
  mode,
  extensions,
  onModeChange,
  onExtensionsChange,
  disabled = false,
  testIdPrefix,
}: {
  mode: UploadFileTypeMode;
  /** 原始输入文本（逗号或空白分隔）；仅在 allow/deny 模式下生效 */
  extensions: string;
  onModeChange: (mode: UploadFileTypeMode) => void;
  onExtensionsChange: (extensions: string) => void;
  disabled?: boolean;
  testIdPrefix: string;
}) {
  const t = useTranslations("room.config.uploadFileType");
  return (
    <div className="space-y-2">
      <Label htmlFor={`${testIdPrefix}-mode`}>{t("label")}</Label>
      <Select
        value={mode}
        onValueChange={(value) => onModeChange(value as UploadFileTypeMode)}
        disabled={disabled}
      >
        <SelectTrigger
          id={`${testIdPrefix}-mode`}
          data-testid={`${testIdPrefix}-mode`}
          className="w-full"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {MODES.map((candidate) => (
            <SelectItem key={candidate} value={candidate}>
              {t(`mode.${candidate}`)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {mode !== "any" && (
        <div className="space-y-1">
          <Label htmlFor={`${testIdPrefix}-extensions`}>{t("extensionsLabel")}</Label>
          <Input
            id={`${testIdPrefix}-extensions`}
            data-testid={`${testIdPrefix}-extensions`}
            value={extensions}
            onChange={(event) => onExtensionsChange(event.target.value)}
            placeholder={t("extensionsPlaceholder")}
            disabled={disabled}
            className="font-mono text-xs"
          />
          <p className="text-xs text-muted-foreground">{t("extensionsHint")}</p>
        </div>
      )}
    </div>
  );
}
