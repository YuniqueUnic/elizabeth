"use client";

import { useCallback, useEffect, useState } from "react";
import { useTranslations, useLocale } from "next-intl";
import { useQuery } from "@tanstack/react-query";

import { CopyButton } from "@/components/copy-button";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  deleteAdminRoom,
  getAdminRoomDetail,
  listAdminRooms,
  mintAdminIdentityCode,
  updateAdminRoom,
} from "@/api/adminService";
import { formatFileSize, formatBackendDateTime, formatDuration } from "@/lib/utils/format";
import { getPublicConfig } from "@/api/publicConfigService";
import {
  UploadFileTypePolicyFields,
  buildUploadFileTypePolicy,
} from "@/components/room/upload-file-type-policy-fields";
import type { UploadFileTypeMode } from "@/lib/types";
import type {
  AdminRoomDetailResponse,
  AdminRoomView,
} from "@/types/generated/api.types";

const PAGE_SIZE = 20;
const ROLE_KEYS = ["admin", "editor", "reader"] as const;
/** 持续时长下拉的「不修改」哨兵值：管理端只提交有改动的字段。 */
const DURATION_UNCHANGED = "unchanged";

function StatusBadge({ status }: { status: AdminRoomView["status"] }) {
  const t = useTranslations("admin");
  const label =
    status === "open"
      ? t("rooms.statusOpen")
      : status === "lock"
        ? t("rooms.statusLock")
        : t("rooms.statusClose");
  return (
    <Badge variant={status === "open" ? "default" : "secondary"}>{label}</Badge>
  );
}

/** 房间管理（容器组件）：默认列表、搜索、分页、配置与删除的数据与副作用。 */
export function AdminRooms({
  adminToken,
  refreshKey,
  onError,
  onDeleted,
  onSaved,
}: {
  adminToken: string;
  /** 递增值触发列表重新加载（面板刷新按钮） */
  refreshKey: number;
  onError: (message: string) => void;
  onDeleted: (name: string) => void;
  onSaved: () => void;
}) {
  const t = useTranslations("admin");
  const [query, setQuery] = useState("");
  const [offset, setOffset] = useState(0);
  const [rooms, setRooms] = useState<AdminRoomView[]>([]);
  const [total, setTotal] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [detail, setDetail] = useState<AdminRoomDetailResponse | null>(null);
  const [pendingDelete, setPendingDelete] = useState<AdminRoomView | null>(null);
  const [deleting, setDeleting] = useState(false);

  const load = useCallback(
    async (nextOffset: number, search = query) => {
      try {
        const result = await listAdminRooms(adminToken, {
          q: search || undefined,
          limit: PAGE_SIZE,
          offset: nextOffset,
        });
        setRooms(result.rooms);
        setTotal(result.total);
        setOffset(nextOffset);
        setLoaded(true);
      } catch (error) {
        onError(error instanceof Error ? error.message : String(error));
      }
    },
    [adminToken, onError, query],
  );

  // 默认列出已有房间；refreshKey 变化（含首次挂载）时重新加载
  useEffect(() => {
    void load(0, "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  async function confirmDelete() {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      await deleteAdminRoom(adminToken, pendingDelete.name);
      setPendingDelete(null);
      onDeleted(pendingDelete.name);
      await load(offset);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setDeleting(false);
    }
  }

  async function openConfig(room: AdminRoomView) {
    try {
      setDetail(await getAdminRoomDetail(adminToken, room.name));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  const totalPages = Math.max(Math.ceil(total / PAGE_SIZE), 1);
  const currentPage = Math.floor(offset / PAGE_SIZE) + 1;

  return (
    <div className="space-y-4">
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          void load(0);
        }}
      >
        <Input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t("rooms.searchPlaceholder")}
          aria-label={t("rooms.searchPlaceholder")}
        />
        <Button type="submit" variant="outline">
          {t("rooms.search")}
        </Button>
      </form>

      <div className="overflow-hidden rounded-md border">
        <div className="bg-muted/50 text-muted-foreground hidden grid-cols-[2fr_1fr_1fr_1fr_1fr_2fr_auto] gap-2 px-4 py-2 text-xs sm:grid">
          <span>{t("rooms.name")}</span>
          <span>{t("rooms.status")}</span>
          <span>{t("rooms.protection")}</span>
          <span>{t("rooms.size")}</span>
          <span>{t("rooms.contents")}</span>
          <span>{t("rooms.expire")}</span>
          <span />
        </div>
        {rooms.map((room) => (
          <div
            key={room.id}
            className="grid grid-cols-2 gap-2 border-t px-4 py-3 text-sm sm:grid-cols-[2fr_1fr_1fr_1fr_1fr_2fr_auto] sm:items-center"
          >
            <span className="font-medium">{room.name}</span>
            <StatusBadge status={room.status} />
            <span>
              {room.password_protected
                ? t("rooms.protected")
                : t("rooms.publicRoom")}
            </span>
            <span>{formatFileSize(room.current_size)}</span>
            <span className="tabular-nums">{room.content_count}</span>
            <span>
              {room.expire_at
                ? formatBackendDateTime(room.expire_at)
                : t("rooms.never")}
            </span>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => void openConfig(room)}
                data-testid="admin-room-configure"
              >
                {t("rooms.configure")}
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={() => setPendingDelete(room)}
              >
                {t("rooms.delete")}
              </Button>
            </div>
          </div>
        ))}
        {loaded && rooms.length === 0 ? (
          <p className="text-muted-foreground p-4 text-center text-sm">
            {t("rooms.empty")}
          </p>
        ) : null}
      </div>

      <div className="flex items-center justify-between">
        <p className="text-muted-foreground text-sm">
          {t("rooms.total", { count: total })}
        </p>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={offset <= 0}
            onClick={() => void load(Math.max(offset - PAGE_SIZE, 0))}
          >
            {t("rooms.prev")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={currentPage >= totalPages}
            onClick={() => void load(offset + PAGE_SIZE)}
          >
            {t("rooms.next")}
          </Button>
        </div>
      </div>

      <RoomConfigDialog
        detail={detail}
        adminToken={adminToken}
        onClose={() => setDetail(null)}
        onError={onError}
        onSaved={(updated) => {
          setDetail(updated);
          onSaved();
          void load(offset);
        }}
      />

      <Dialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("rooms.deleteConfirmTitle")}</DialogTitle>
            <DialogDescription>
              {t("rooms.deleteConfirmDescription", {
                name: pendingDelete?.name ?? "",
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPendingDelete(null)}>
              {t("rooms.cancel")}
            </Button>
            <Button
              variant="destructive"
              disabled={deleting}
              onClick={() => void confirmDelete()}
            >
              {t("rooms.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

/** 房间配置对话框（展示组件）：概览 / 设置 / 身份码 三个分区。 */
function RoomConfigDialog({
  detail,
  adminToken,
  onClose,
  onError,
  onSaved,
}: {
  detail: AdminRoomDetailResponse | null;
  adminToken: string;
  onClose: () => void;
  onError: (message: string) => void;
  onSaved: (detail: AdminRoomDetailResponse) => void;
}) {
  const t = useTranslations("admin");
  const tUploadFileType = useTranslations("room.config.uploadFileType");
  const locale = useLocale();
  const [maxSize, setMaxSize] = useState("");
  const [maxTimes, setMaxTimes] = useState("");
  const [defaultRole, setDefaultRole] = useState("");
  const [duration, setDuration] = useState<number | null>(null);
  const [uploadMode, setUploadMode] = useState<UploadFileTypeMode>("any");
  const [uploadExtensions, setUploadExtensions] = useState("");
  const [password, setPassword] = useState("");
  const [removePassword, setRemovePassword] = useState(false);
  const [saving, setSaving] = useState(false);
  const [mintRole, setMintRole] = useState<(typeof ROLE_KEYS)[number]>("editor");
  const [mintCode, setMintCode] = useState("");
  const [mintedCode, setMintedCode] = useState<string | null>(null);
  const [minting, setMinting] = useState(false);
  // 允许的时长属于部署配置，与房间侧共用同一份公开配置，避免策略出现两个来源。
  const config = useQuery({
    queryKey: ["public-config"],
    queryFn: getPublicConfig,
    staleTime: Infinity,
    enabled: detail !== null,
  });
  const allowedAges = config.data?.room.expiry.allowed_ages_seconds ?? [];

  useEffect(() => {
    if (detail) {
      setMaxSize("");
      setMaxTimes("");
      setDefaultRole("");
      setDuration(null);
      setUploadMode(detail.upload_file_type.mode);
      setUploadExtensions(detail.upload_file_type.extensions.join(", "));
      setPassword("");
      setRemovePassword(false);
      setMintRole("editor");
      setMintCode("");
      setMintedCode(null);
    }
  }, [detail]);

  if (!detail) return null;

  const uploadPolicy = buildUploadFileTypePolicy(uploadMode, uploadExtensions);
  const uploadPolicyChanged =
    uploadMode !== detail.upload_file_type.mode ||
    uploadPolicy.extensions.join(",") !== detail.upload_file_type.extensions.join(",");

  async function saveSettings() {
    if (!detail || saving) return;
    setSaving(true);
    try {
      const update: Parameters<typeof updateAdminRoom>[2] = {};
      if (maxSize !== "") update.max_size = Number(maxSize);
      if (maxTimes !== "") update.max_times_entered = Number(maxTimes);
      if (defaultRole !== "") update.default_role_key = defaultRole;
      if (duration !== null) update.age_seconds = duration;
      if (uploadPolicyChanged) update.upload_file_type = uploadPolicy;
      if (removePassword) {
        update.remove_password = true;
      } else if (password !== "") {
        update.password = password;
      }
      const updated = await updateAdminRoom(adminToken, detail.name, update);
      onSaved(updated);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  async function mint() {
    if (!detail || minting) return;
    setMinting(true);
    try {
      const result = await mintAdminIdentityCode(adminToken, detail.name, {
        code: mintCode || undefined,
        role: mintRole,
      });
      setMintedCode(result.code);
      setMintCode("");
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setMinting(false);
    }
  }

  const hasSettingChanges =
    maxSize !== "" ||
    maxTimes !== "" ||
    defaultRole !== "" ||
    duration !== null ||
    uploadPolicyChanged ||
    password !== "" ||
    removePassword;

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {t("rooms.detailTitle")}: {detail.name}
          </DialogTitle>
          <DialogDescription>{detail.slug}</DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="overview">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="overview">{t("rooms.tabOverview")}</TabsTrigger>
            <TabsTrigger value="settings">{t("rooms.tabSettings")}</TabsTrigger>
            <TabsTrigger value="identity">{t("rooms.tabIdentity")}</TabsTrigger>
          </TabsList>

          <TabsContent value="overview" className="mt-3">
            <dl className="grid grid-cols-2 gap-2 text-sm">
              <dt className="text-muted-foreground">{t("rooms.contents")}</dt>
              <dd className="tabular-nums">{detail.content_count}</dd>
              <dt className="text-muted-foreground">{t("rooms.detailBlobs")}</dt>
              <dd className="tabular-nums">{detail.blob_count}</dd>
              <dt className="text-muted-foreground">{t("rooms.detailTokens")}</dt>
              <dd className="tabular-nums">{detail.token_count}</dd>
              <dt className="text-muted-foreground">{t("rooms.size")}</dt>
              <dd>
                {formatFileSize(detail.current_size)} /{" "}
                {formatFileSize(detail.max_size)}
              </dd>
              <dt className="text-muted-foreground">
                {t("rooms.overviewEntries")}
              </dt>
              <dd className="tabular-nums">
                {detail.current_times_entered} / {detail.max_times_entered}
              </dd>
              <dt className="text-muted-foreground">{t("rooms.defaultRole")}</dt>
              <dd>{detail.default_role_key}</dd>
              <dt className="text-muted-foreground">{t("rooms.uploadFileType")}</dt>
              <dd>
                {tUploadFileType(`mode.${detail.upload_file_type.mode}`)}
                {detail.upload_file_type.extensions.length > 0
                  ? ` (${detail.upload_file_type.extensions.join(", ")})`
                  : ""}
              </dd>
              <dt className="text-muted-foreground">{t("rooms.created")}</dt>
              <dd>{formatBackendDateTime(detail.created_at)}</dd>
              <dt className="text-muted-foreground">{t("rooms.expire")}</dt>
              <dd>
                {detail.expire_at
                  ? formatBackendDateTime(detail.expire_at)
                  : t("rooms.never")}
              </dd>
            </dl>
          </TabsContent>

          <TabsContent value="settings" className="mt-3 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1">
                <Label htmlFor="admin-room-max-size">{t("rooms.maxSize")}</Label>
                <Input
                  id="admin-room-max-size"
                  inputMode="numeric"
                  placeholder={String(detail.max_size)}
                  value={maxSize}
                  onChange={(event) => setMaxSize(event.target.value)}
                />
                <p className="text-muted-foreground text-xs">
                  {t("rooms.capacityCurrent", {
                    size: formatFileSize(detail.max_size),
                    bytes: detail.max_size,
                  })}
                </p>
              </div>
              <div className="space-y-1">
                <Label htmlFor="admin-room-max-times">
                  {t("rooms.maxTimes")}
                </Label>
                <Input
                  id="admin-room-max-times"
                  inputMode="numeric"
                  placeholder={String(detail.max_times_entered)}
                  value={maxTimes}
                  onChange={(event) => setMaxTimes(event.target.value)}
                />
                <p className="text-muted-foreground text-xs">
                  {t("rooms.maxTimesCurrent", {
                    count: detail.max_times_entered,
                  })}
                </p>
              </div>
            </div>

            <div className="space-y-1">
              <Label>{t("rooms.defaultRole")}</Label>
              <Select
                value={defaultRole || detail.default_role_key}
                onValueChange={(value) =>
                  setDefaultRole(value === detail.default_role_key ? "" : value)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ROLE_KEYS.map((role) => (
                    <SelectItem key={role} value={role}>
                      {role}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-muted-foreground text-xs">
                {t("rooms.roleCurrent", { role: detail.default_role_key })}
              </p>
            </div>

            <div className="space-y-1">
              <Label htmlFor="admin-room-duration">{t("rooms.duration")}</Label>
              <Select
                value={duration === null ? DURATION_UNCHANGED : String(duration)}
                onValueChange={(value) =>
                  setDuration(value === DURATION_UNCHANGED ? null : Number(value))
                }
                disabled={allowedAges.length === 0}
              >
                <SelectTrigger
                  id="admin-room-duration"
                  className="w-full"
                  data-testid="admin-room-duration"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={DURATION_UNCHANGED}>
                    {t("rooms.durationUnchanged")}
                  </SelectItem>
                  {allowedAges.map((ageSeconds) => (
                    <SelectItem
                      key={ageSeconds}
                      value={String(ageSeconds)}
                      data-testid={`admin-room-duration-option-${ageSeconds}`}
                    >
                      {formatDuration(ageSeconds, locale)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-muted-foreground text-xs">
                {detail.expire_at
                  ? t("rooms.durationCurrent", {
                      time: formatBackendDateTime(detail.expire_at),
                    })
                  : t("rooms.never")}
              </p>
            </div>

            <UploadFileTypePolicyFields
              mode={uploadMode}
              extensions={uploadExtensions}
              onModeChange={setUploadMode}
              onExtensionsChange={setUploadExtensions}
              testIdPrefix="admin-room-upload-file-type"
            />

            <div className="space-y-1">
              <Label htmlFor="admin-room-password">
                {t("rooms.passwordSet")}
              </Label>
              <Input
                id="admin-room-password"
                type="password"
                placeholder={t("rooms.passwordPlaceholder")}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                disabled={removePassword}
              />
              {detail.password_protected ? (
                <label className="text-muted-foreground flex items-center gap-2 text-xs">
                  <input
                    type="checkbox"
                    checked={removePassword}
                    onChange={(event) => {
                      setRemovePassword(event.target.checked);
                      if (event.target.checked) setPassword("");
                    }}
                  />
                  {t("rooms.passwordRemove")}
                </label>
              ) : (
                <p className="text-muted-foreground text-xs">
                  {t("rooms.passwordAbsent")}
                </p>
              )}
            </div>

            <p className="text-muted-foreground text-xs">
              {t("rooms.settingsHint")}
            </p>
            <Button
              size="sm"
              disabled={!hasSettingChanges || saving}
              onClick={() => void saveSettings()}
              data-testid="admin-room-save"
            >
              {t("rooms.saveSettings")}
            </Button>
          </TabsContent>

          <TabsContent value="identity" className="mt-3 space-y-3">
            {mintedCode ? (
              <div
                className="flex items-center gap-2 border border-primary/40 bg-primary/5 rounded-md p-2"
                data-testid="minted-code"
              >
                <code className="flex-1 font-mono text-xs break-all">
                  {mintedCode}
                </code>
                <CopyButton value={mintedCode} label={t("rooms.copyMinted")} />
              </div>
            ) : (
              <p className="text-muted-foreground text-xs">
                {t("rooms.mintHint")}
              </p>
            )}
            <div className="space-y-1">
              <Label>{t("rooms.mintRole")}</Label>
              <Select value={mintRole} onValueChange={(v) => setMintRole(v as (typeof ROLE_KEYS)[number])}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ROLE_KEYS.map((role) => (
                    <SelectItem key={role} value={role}>
                      {role}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <Label htmlFor="admin-mint-code">{t("rooms.mintCode")}</Label>
              <Input
                id="admin-mint-code"
                className="font-mono text-xs"
                placeholder={t("rooms.mintCodePlaceholder")}
                value={mintCode}
                onChange={(event) => setMintCode(event.target.value)}
              />
            </div>
            <Button
              size="sm"
              variant="outline"
              disabled={minting || mintedCode !== null}
              onClick={() => void mint()}
              data-testid="admin-room-mint"
            >
              {t("rooms.mintButton")}
            </Button>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
