"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";

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
import { deleteAdminRoom, getAdminRoomDetail, listAdminRooms } from "@/api/adminService";
import { formatFileSize } from "@/lib/utils/format";
import type {
  AdminRoomDetailResponse,
  AdminRoomView,
} from "@/types/generated/api.types";

const PAGE_SIZE = 20;

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

/** 房间管理（容器组件）：搜索、分页、详情与删除的数据与副作用。 */
export function AdminRooms({
  adminToken,
  onError,
  onDeleted,
}: {
  adminToken: string;
  onError: (message: string) => void;
  onDeleted: (name: string) => void;
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

  async function load(nextOffset: number, search = query) {
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
  }

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

      <Dialog open={detail !== null} onOpenChange={(open) => !open && setDetail(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("rooms.detailTitle")}: {detail?.name}
            </DialogTitle>
            <DialogDescription>{detail?.slug}</DialogDescription>
          </DialogHeader>
          {detail ? (
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
          ) : null}
        </DialogContent>
      </Dialog>

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
