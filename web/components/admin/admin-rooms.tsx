"use client";

import { useCallback, useEffect, useState } from "react";
import { useTranslations } from "next-intl";

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
  deleteAdminRoom,
  getAdminRoomDetail,
  listAdminRooms,
  mintAdminIdentityCode,
  updateAdminRoom,
} from "@/api/adminService";
import { formatFileSize } from "@/lib/utils/format";
import type {
  AdminRoomDetailResponse,
  AdminRoomView,
} from "@/types/generated/api.types";

const PAGE_SIZE = 20;
const ROLE_KEYS = ["admin", "editor", "reader"] as const;

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

/** 房间管理（容器组件）：默认列表、搜索、分页、详情/设置编辑、身份码铸造与删除。 */
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

  async function openDetail(room: AdminRoomView) {
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
            <button
              type="button"
              className="text-left font-medium underline-offset-4 hover:underline"
              onClick={() => void openDetail(room)}
            >
              {room.name}
            </button>
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
                ? new Date(room.expire_at).toLocaleString()
                : t("rooms.never")}
            </span>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => setPendingDelete(room)}
            >
              {t("rooms.delete")}
            </Button>
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

      <RoomDetailDialog
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

/** 房间详情 + 设置编辑 + 身份码铸造（展示组件，数据由父级加载）。 */
function RoomDetailDialog({
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
  // 设置表单：null/空串 = 保持不变（服务端按字段缺省处理）
  const [maxSize, setMaxSize] = useState("");
  const [maxTimes, setMaxTimes] = useState("");
  const [defaultRole, setDefaultRole] = useState("");
  const [password, setPassword] = useState("");
  const [removePassword, setRemovePassword] = useState(false);
  const [saving, setSaving] = useState(false);

  // 身份码铸造：角色 + 可选指定码；铸造结果明文仅此一次展示
  const [mintRole, setMintRole] = useState<(typeof ROLE_KEYS)[number]>("editor");
  const [mintCode, setMintCode] = useState("");
  const [mintedCode, setMintedCode] = useState<string | null>(null);
  const [minting, setMinting] = useState(false);

  useEffect(() => {
    if (detail) {
      setMaxSize("");
      setMaxTimes("");
      setDefaultRole("");
      setPassword("");
      setRemovePassword(false);
      setMintRole("editor");
      setMintCode("");
      setMintedCode(null);
    }
  }, [detail]);

  if (!detail) return null;

  async function saveSettings() {
    if (!detail) return;
    setSaving(true);
    try {
      const update: Parameters<typeof updateAdminRoom>[2] = {};
      if (maxSize !== "") update.max_size = Number(maxSize);
      if (maxTimes !== "") update.max_times_entered = Number(maxTimes);
      if (defaultRole !== "") update.default_role_key = defaultRole;
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

        <dl className="grid grid-cols-2 gap-2 text-sm">
          <dt>{t("rooms.contents")}</dt>
          <dd className="tabular-nums">{detail.content_count}</dd>
          <dt>{t("rooms.detailBlobs")}</dt>
          <dd className="tabular-nums">{detail.blob_count}</dd>
          <dt>{t("rooms.detailTokens")}</dt>
          <dd className="tabular-nums">{detail.token_count}</dd>
          <dt>{t("rooms.size")}</dt>
          <dd>{formatFileSize(detail.current_size)}</dd>
          <dt>{t("rooms.created")}</dt>
          <dd>{new Date(detail.created_at).toLocaleString()}</dd>
        </dl>

        <div className="space-y-2 border-t pt-3">
          <p className="text-sm font-medium">{t("rooms.settingsTitle")}</p>
          <div className="grid grid-cols-2 gap-2">
            <div className="space-y-1">
              <Label htmlFor="admin-room-max-size">{t("rooms.maxSize")}</Label>
              <Input
                id="admin-room-max-size"
                inputMode="numeric"
                placeholder={String(detail.max_size)}
                value={maxSize}
                onChange={(event) => setMaxSize(event.target.value)}
              />
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
            </div>
          </div>
          <div className="space-y-1">
            <Label htmlFor="admin-room-default-role">
              {t("rooms.defaultRole")}
            </Label>
            <select
              id="admin-room-default-role"
              className="border-input bg-background h-9 w-full rounded-md border px-2 text-sm"
              value={defaultRole || detail.default_role_key}
              onChange={(event) => setDefaultRole(event.target.value)}
            >
              {ROLE_KEYS.map((role) => (
                <option key={role} value={role}>
                  {role}
                </option>
              ))}
            </select>
          </div>
          <div className="grid grid-cols-2 items-end gap-2">
            <div className="space-y-1">
              <Label htmlFor="admin-room-password">
                {t("rooms.passwordSet")}
              </Label>
              <Input
                id="admin-room-password"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                disabled={removePassword}
              />
            </div>
            <label className="text-muted-foreground flex items-center gap-2 text-sm">
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
        </div>

        <div className="space-y-2 border-t pt-3">
          <p className="text-sm font-medium">{t("rooms.mintTitle")}</p>
          {mintedCode ? (
            <div className="flex items-center gap-2" data-testid="minted-code">
              <code className="bg-muted flex-1 rounded px-2 py-1 font-mono text-xs">
                {mintedCode}
              </code>
              <CopyButton value={mintedCode} label={t("rooms.copyMinted")} />
            </div>
          ) : (
            <p className="text-muted-foreground text-xs">{t("rooms.mintHint")}</p>
          )}
          <div className="grid grid-cols-2 items-end gap-2">
            <div className="space-y-1">
              <Label htmlFor="admin-mint-role">{t("rooms.mintRole")}</Label>
              <select
                id="admin-mint-role"
                className="border-input bg-background h-9 w-full rounded-md border px-2 text-sm"
                value={mintRole}
                onChange={(event) =>
                  setMintRole(event.target.value as (typeof ROLE_KEYS)[number])
                }
              >
                {ROLE_KEYS.map((role) => (
                  <option key={role} value={role}>
                    {role}
                  </option>
                ))}
              </select>
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
        </div>
      </DialogContent>
    </Dialog>
  );
}
