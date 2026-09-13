"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";

import {
  updateAdminCredential,
  updateRuntimeConfig,
} from "@/api/adminService";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatFileSize } from "@/lib/utils/format";
import type {
  AdminConfigResponse,
  AdminStorageResponse,
} from "@/types/generated/api.types";

const RUNTIME_ROLE_KEYS = ["admin", "editor", "reader"] as const;

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-1.5">
      <span className="text-muted-foreground text-sm">{label}</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  );
}

/** 系统 tab（容器组件）：存储状态 + 运行时可写白名单 + 管理凭证轮换 + 只读配置展示。 */
export function AdminSystem({
  storage,
  config,
  adminToken,
  onError,
  onSaved,
  onCredentialRotated,
}: {
  storage: AdminStorageResponse;
  config: AdminConfigResponse;
  adminToken: string;
  onError: (message: string) => void;
  onSaved: (updated: AdminConfigResponse) => void;
  /** 凭证轮换成功后同步父级会话（sessionStorage + 请求头），避免持有已轮换的旧值 */
  onCredentialRotated: (newToken: string) => void;
}) {
  const t = useTranslations("admin");
  const [saving, setSaving] = useState(false);
  const [maxSize, setMaxSize] = useState(
    config.runtime_room_default_max_size > 0
      ? String(config.runtime_room_default_max_size)
      : "",
  );
  const [maxTimes, setMaxTimes] = useState(
    config.runtime_room_default_max_times_entered > 0
      ? String(config.runtime_room_default_max_times_entered)
      : "",
  );
  const [sessionTtl, setSessionTtl] = useState(
    config.runtime_session_ttl_seconds > 0
      ? String(config.runtime_session_ttl_seconds)
      : "",
  );
  const [reservationTtl, setReservationTtl] = useState(
    config.runtime_upload_reservation_ttl_seconds > 0
      ? String(config.runtime_upload_reservation_ttl_seconds)
      : "",
  );
  const [defaultRole, setDefaultRole] = useState(
    config.runtime_room_default_role_key ?? "",
  );
  const [newAdminToken, setNewAdminToken] = useState("");
  const [rotating, setRotating] = useState(false);

  async function saveRuntime() {
    setSaving(true);
    try {
      const updated = await updateRuntimeConfig(adminToken, {
        room_default_max_size: maxSize === "" ? 0 : Number(maxSize),
        room_default_max_times_entered: maxTimes === "" ? 0 : Number(maxTimes),
        session_ttl_seconds: sessionTtl === "" ? 0 : Number(sessionTtl),
        upload_reservation_ttl_seconds:
          reservationTtl === "" ? 0 : Number(reservationTtl),
        room_default_role_key: defaultRole,
      });
      onSaved(updated);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  async function toggleSearchIndexing() {
    setSaving(true);
    try {
      const updated = await updateRuntimeConfig(adminToken, {
        disallow_search_indexing: !config.runtime_disallow_search_indexing,
      });
      onSaved(updated);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  async function rotateCredential() {
    if (newAdminToken === "") return;
    setRotating(true);
    try {
      await updateAdminCredential(adminToken, newAdminToken);
      onCredentialRotated(newAdminToken);
      setNewAdminToken("");
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setRotating(false);
    }
  }

  const numeric = (value: string) => /^\d+$/.test(value) || value === "";
  const invalid =
    !numeric(maxSize) ||
    !numeric(maxTimes) ||
    !numeric(sessionTtl) ||
    !numeric(reservationTtl);

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>{t("system.storageTitle")}</CardTitle>
        </CardHeader>
        <CardContent className="divide-y">
          <Row
            label={t("system.backend")}
            value={
              <Badge variant="secondary">
                {storage.backend === "s3"
                  ? t("system.backendS3")
                  : t("system.backendFs")}
              </Badge>
            }
          />
          <Row
            label={t("system.transferMode")}
            value={
              <Badge variant="secondary">
                {storage.transfer_mode === "presigned"
                  ? t("system.transferPresigned")
                  : t("system.transferProxy")}
              </Badge>
            }
          />
          {storage.root ? (
            <Row label={t("system.root")} value={storage.root} />
          ) : null}
          {storage.bucket ? (
            <Row label={t("system.bucket")} value={storage.bucket} />
          ) : null}
          {storage.presign_base_url ? (
            <Row label={t("system.presignBase")} value={storage.presign_base_url} />
          ) : null}
          <Row
            label={t("system.physicalBytes")}
            value={formatFileSize(storage.physical_bytes)}
          />
          <Row
            label={t("system.logicalBytes")}
            value={formatFileSize(storage.logical_bytes)}
          />
          <Row
            label={t("system.dedupSaved")}
            value={formatFileSize(storage.dedup_saved_bytes)}
          />
          <Row
            label={t("system.dedupScope")}
            value={<Badge variant="secondary">{config.dedup_scope}</Badge>}
          />
        </CardContent>
      </Card>

      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>{t("system.runtimeTitle")}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="admin-search-indexing">{t("system.searchIndexing")}</Label>
                <p className="text-muted-foreground text-xs">
                  {config.runtime_disallow_search_indexing
                    ? t("system.searchIndexingDisallowed")
                    : t("system.searchIndexingAllowed")}
                </p>
              </div>
              <Button
                id="admin-search-indexing"
                data-testid="admin-toggle-search-indexing"
                variant={config.runtime_disallow_search_indexing ? "destructive" : "outline"}
                size="sm"
                disabled={saving}
                onClick={() => void toggleSearchIndexing()}
              >
                {config.runtime_disallow_search_indexing
                  ? t("system.searchIndexingDisallowed")
                  : t("system.searchIndexingAllowed")}
              </Button>
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-2">
                <Label htmlFor="admin-runtime-max-size">{t("system.runtimeMaxSize")}</Label>
                <Input
                  id="admin-runtime-max-size"
                  inputMode="numeric"
                  placeholder={String(config.room_default_max_size)}
                  value={maxSize}
                  onChange={(event) => setMaxSize(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="admin-runtime-max-times">{t("system.runtimeMaxTimes")}</Label>
                <Input
                  id="admin-runtime-max-times"
                  inputMode="numeric"
                  placeholder={String(config.room_default_max_times_entered)}
                  value={maxTimes}
                  onChange={(event) => setMaxTimes(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="admin-runtime-session-ttl">
                  {t("system.runtimeSessionTtl")}
                </Label>
                <Input
                  id="admin-runtime-session-ttl"
                  inputMode="numeric"
                  placeholder={String(config.jwt_ttl_seconds)}
                  value={sessionTtl}
                  onChange={(event) => setSessionTtl(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="admin-runtime-reservation-ttl">
                  {t("system.runtimeReservationTtl")}
                </Label>
                <Input
                  id="admin-runtime-reservation-ttl"
                  inputMode="numeric"
                  placeholder={String(config.upload_reservation_ttl_seconds)}
                  value={reservationTtl}
                  onChange={(event) => setReservationTtl(event.target.value)}
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="admin-runtime-default-role">
                {t("system.runtimeDefaultRole")}
              </Label>
              <select
                id="admin-runtime-default-role"
                data-testid="admin-runtime-default-role"
                className="border-input bg-background h-9 w-full rounded-md border px-2 text-sm"
                value={defaultRole}
                onChange={(event) => setDefaultRole(event.target.value)}
              >
                <option value="">
                  {t("system.runtimeDefaultRoleFollow", {
                    role: config.room_default_role_key,
                  })}
                </option>
                {RUNTIME_ROLE_KEYS.map((role) => (
                  <option key={role} value={role}>
                    {role}
                  </option>
                ))}
              </select>
              <p className="text-muted-foreground text-xs">
                {t("system.runtimeDefaultRoleHint")}
              </p>
            </div>
            <div className="flex items-center justify-between gap-4">
              <p className="text-muted-foreground text-xs">{t("system.runtimeHint")}</p>
              <Button
                size="sm"
                data-testid="admin-save-runtime"
                disabled={saving || invalid}
                onClick={() => void saveRuntime()}
              >
                {t("system.save")}
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("system.credentialTitle")}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <Row
              label={t("system.credentialSource")}
              value={
                <Badge variant="secondary" data-testid="admin-credential-source">
                  {config.admin_token_source}
                </Badge>
              }
            />
            <div className="space-y-2">
              <Label htmlFor="admin-credential-new">{t("system.credentialNew")}</Label>
              <Input
                id="admin-credential-new"
                type="password"
                data-testid="admin-credential-new"
                placeholder={t("system.credentialPlaceholder")}
                value={newAdminToken}
                onChange={(event) => setNewAdminToken(event.target.value)}
              />
              <p className="text-muted-foreground text-xs">
                {t("system.credentialHint")}
              </p>
            </div>
            <Button
              size="sm"
              data-testid="admin-rotate-credential"
              disabled={rotating || newAdminToken === ""}
              onClick={() => void rotateCredential()}
            >
              {t("system.credentialRotate")}
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("system.configTitle")}</CardTitle>
          </CardHeader>
          <CardContent className="divide-y">
            <Row
              label={t("system.server")}
              value={`${config.server_host}:${config.server_port}`}
            />
            <Row label={t("system.database")} value={config.database_backend} />
            <Row
              label={t("system.maxSize")}
              value={formatFileSize(config.room_default_max_size)}
            />
            <Row
              label={t("system.maxTimesEntered")}
              value={config.room_default_max_times_entered}
            />
            <Row label={t("system.defaultRole")} value={config.room_default_role_key} />
            <Row label={t("system.jwtTtl")} value={`${config.jwt_ttl_seconds}s`} />
            <Row
              label={t("system.jwtRefreshTtl")}
              value={`${config.jwt_refresh_ttl_seconds}s`}
            />
            <Row
              label={t("system.reservationTtl")}
              value={`${config.upload_reservation_ttl_seconds}s`}
            />
            <Row
              label={t("system.adminApiEnabled")}
              value={
                config.admin_api_enabled ? (
                  <Badge>{t("system.yes")}</Badge>
                ) : (
                  <Badge variant="secondary">{t("system.no")}</Badge>
                )
              }
            />
            <p className="text-muted-foreground pt-2 text-xs">{t("system.readonlyHint")}</p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
