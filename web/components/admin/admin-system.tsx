"use client";

import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";

import {
  adminChangePassword,
  createAdminApiKey,
  listAdminApiKeys,
  revokeAdminApiKey,
  updateRuntimeConfig,
} from "@/api/adminService";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatFileSize, formatDurationList, parseDurationList } from "@/lib/utils/format";
import type {
  AdminApiKeyView,
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

/** 系统 tab（容器组件）：存储状态 + 运行时可写白名单 + 账号密码 / API key 管理 + 只读配置展示。 */
export function AdminSystem({
  storage,
  config,
  sessionToken,
  onError,
  onSaved,
  /** 改密使当前会话一并失效，父组件据此回到登录页 */
  onPasswordChanged,
}: {
  storage: AdminStorageResponse;
  config: AdminConfigResponse;
  sessionToken: string;
  onError: (message: string) => void;
  onSaved: (updated: AdminConfigResponse) => void;
  onPasswordChanged: () => void;
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
  // 有效期策略是「允许时长 + 默认时长」的一组，编辑器与配置文件同形（1m/2h/7d）。
  const configExpiryAges = formatDurationList(config.room_expiry_allowed_ages_seconds);
  const configExpiryDefault = formatDurationList([config.room_expiry_default_age_seconds]);
  const [expiryAges, setExpiryAges] = useState(
    formatDurationList(
      config.runtime_room_expiry?.allowed_ages_seconds ??
        config.room_expiry_allowed_ages_seconds,
    ),
  );
  const [expiryDefault, setExpiryDefault] = useState(
    formatDurationList([
      config.runtime_room_expiry?.default_age_seconds ?? config.room_expiry_default_age_seconds,
    ]),
  );

  const parsedExpiryAges = parseDurationList(expiryAges);
  const parsedExpiryDefault = parseDurationList(expiryDefault);
  const expiryChanged =
    expiryAges.trim() !==
      formatDurationList(
        config.runtime_room_expiry?.allowed_ages_seconds ??
          config.room_expiry_allowed_ages_seconds,
      ) ||
    expiryDefault.trim() !==
      formatDurationList([
        config.runtime_room_expiry?.default_age_seconds ?? config.room_expiry_default_age_seconds,
      ]);

  async function saveRuntime() {
    setSaving(true);
    try {
      const update: Parameters<typeof updateRuntimeConfig>[1] = {
        room_default_max_size: maxSize === "" ? 0 : Number(maxSize),
        room_default_max_times_entered: maxTimes === "" ? 0 : Number(maxTimes),
        session_ttl_seconds: sessionTtl === "" ? 0 : Number(sessionTtl),
        upload_reservation_ttl_seconds:
          reservationTtl === "" ? 0 : Number(reservationTtl),
        room_default_role_key: defaultRole,
      };
      // 未改动时不提交，避免把「跟随配置文件」静默固化成一份等价覆盖。
      if (expiryChanged && parsedExpiryAges && parsedExpiryDefault?.length === 1) {
        update.room_expiry = {
          allowed_ages_seconds: parsedExpiryAges,
          default_age_seconds: parsedExpiryDefault[0],
        };
      }
      const updated = await updateRuntimeConfig(sessionToken, update);
      onSaved(updated);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  async function clearRoomExpiryOverride() {
    setSaving(true);
    try {
      const updated = await updateRuntimeConfig(sessionToken, {
        room_expiry: { allowed_ages_seconds: [], default_age_seconds: 0 },
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
      const updated = await updateRuntimeConfig(sessionToken, {
        disallow_search_indexing: !config.runtime_disallow_search_indexing,
      });
      onSaved(updated);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  const numeric = (value: string) => /^\d+$/.test(value) || value === "";
  const expiryInputInvalid =
    parsedExpiryAges === null ||
    parsedExpiryDefault === null ||
    parsedExpiryDefault.length !== 1;
  const invalid =
    !numeric(maxSize) ||
    !numeric(maxTimes) ||
    !numeric(sessionTtl) ||
    !numeric(reservationTtl) ||
    (expiryChanged && expiryInputInvalid);

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
            <div className="space-y-2 border-t pt-3">
              <div className="flex items-center justify-between gap-4">
                <p className="text-sm font-medium">{t("system.runtimeRoomExpiry")}</p>
                <Button
                  variant="outline"
                  size="sm"
                  data-testid="admin-clear-room-expiry"
                  disabled={saving || config.runtime_room_expiry === null}
                  onClick={() => void clearRoomExpiryOverride()}
                >
                  {t("system.runtimeRoomExpiryClear")}
                </Button>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-2">
                  <Label htmlFor="admin-runtime-expiry-ages">
                    {t("system.runtimeRoomExpiryAllowed")}
                  </Label>
                  <Input
                    id="admin-runtime-expiry-ages"
                    data-testid="admin-runtime-expiry-ages"
                    className="font-mono text-xs"
                    placeholder={configExpiryAges}
                    value={expiryAges}
                    onChange={(event) => setExpiryAges(event.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="admin-runtime-expiry-default">
                    {t("system.runtimeRoomExpiryDefault")}
                  </Label>
                  <Input
                    id="admin-runtime-expiry-default"
                    data-testid="admin-runtime-expiry-default"
                    className="font-mono text-xs"
                    placeholder={configExpiryDefault}
                    value={expiryDefault}
                    onChange={(event) => setExpiryDefault(event.target.value)}
                  />
                </div>
              </div>
              <p className="text-muted-foreground text-xs">
                {t("system.runtimeRoomExpiryHint")}
              </p>
              {config.runtime_room_expiry === null && (
                <p className="text-muted-foreground text-xs" data-testid="admin-room-expiry-base">
                  {t("system.runtimeRoomExpiryFollowing", { ages: configExpiryAges })}
                </p>
              )}
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

        <AdminPasswordCard
          sessionToken={sessionToken}
          onError={onError}
          onPasswordChanged={onPasswordChanged}
        />
        <AdminApiKeysCard sessionToken={sessionToken} onError={onError} />

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
            <Row
              label={t("system.roomExpiry")}
              value={`${configExpiryAges} · ${configExpiryDefault}`}
            />
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

/** 管理员账号密码修改：改密后该账号所有会话失效（含当前会话，由父组件登出）。 */
function AdminPasswordCard({
  sessionToken,
  onError,
  onPasswordChanged,
}: {
  sessionToken: string;
  onError: (message: string) => void;
  onPasswordChanged: () => void;
}) {
  const t = useTranslations("admin");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [changing, setChanging] = useState(false);

  const mismatch = confirmPassword !== "" && confirmPassword !== newPassword;
  const ready = currentPassword !== "" && newPassword !== "" && !mismatch;

  async function changePassword() {
    setChanging(true);
    try {
      await adminChangePassword(sessionToken, {
        current_password: currentPassword,
        new_password: newPassword,
      });
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      onPasswordChanged();
    } catch (error) {
      // 改密失败（含当前密码错误）原样呈现；当前会话仍有效
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setChanging(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("system.passwordTitle")}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="space-y-2">
          <Label htmlFor="admin-password-current">{t("system.passwordCurrent")}</Label>
          <Input
            id="admin-password-current"
            type="password"
            autoComplete="current-password"
            data-testid="admin-password-current"
            value={currentPassword}
            onChange={(event) => setCurrentPassword(event.target.value)}
          />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div className="space-y-2">
            <Label htmlFor="admin-password-new">{t("system.passwordNew")}</Label>
            <Input
              id="admin-password-new"
              type="password"
              autoComplete="new-password"
              data-testid="admin-password-new"
              value={newPassword}
              onChange={(event) => setNewPassword(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-password-confirm">{t("system.passwordConfirm")}</Label>
            <Input
              id="admin-password-confirm"
              type="password"
              autoComplete="new-password"
              data-testid="admin-password-confirm"
              value={confirmPassword}
              onChange={(event) => setConfirmPassword(event.target.value)}
            />
            {mismatch && (
              <p className="text-destructive text-xs">{t("system.passwordMismatch")}</p>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between gap-4">
          <p className="text-muted-foreground text-xs">{t("system.passwordHint")}</p>
          <Button
            size="sm"
            data-testid="admin-change-password"
            disabled={changing || !ready}
            onClick={() => void changePassword()}
          >
            {t("system.passwordChange")}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

/** 管理 API key：创建（明文一次性展示）、列表与吊销。 */
function AdminApiKeysCard({
  sessionToken,
  onError,
}: {
  sessionToken: string;
  onError: (message: string) => void;
}) {
  const t = useTranslations("admin");
  const [keys, setKeys] = useState<AdminApiKeyView[] | null>(null);
  const [name, setName] = useState("");
  const [expiresIn, setExpiresIn] = useState("");
  const [creating, setCreating] = useState(false);
  /** 刚创建的明文 key：仅在本次会话中展示一次 */
  const [freshSecret, setFreshSecret] = useState<{ name: string; secret: string } | null>(null);

  const reload = useCallback(async () => {
    try {
      setKeys(await listAdminApiKeys(sessionToken));
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }, [sessionToken, onError]);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function createKey() {
    setCreating(true);
    try {
      const created = await createAdminApiKey(sessionToken, {
        name: name.trim(),
        expires_in_secs: expiresIn === "" ? undefined : Number(expiresIn),
      });
      setFreshSecret({ name: created.name, secret: created.secret });
      setName("");
      setExpiresIn("");
      await reload();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    } finally {
      setCreating(false);
    }
  }

  async function revokeKey(id: number) {
    try {
      await revokeAdminApiKey(sessionToken, id);
      await reload();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  const EXPIRY_CHOICES = [
    { value: "", label: t("system.apiKeyNeverExpires") },
    { value: "86400", label: t("system.apiKeyExpiryDay") },
    { value: "604800", label: t("system.apiKeyExpiryWeek") },
    { value: "2592000", label: t("system.apiKeyExpiryMonth") },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("system.apiKeyTitle")}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-[1fr_auto_auto] items-end gap-2">
          <div className="space-y-2">
            <Label htmlFor="admin-api-key-name">{t("system.apiKeyName")}</Label>
            <Input
              id="admin-api-key-name"
              data-testid="admin-api-key-name"
              placeholder={t("system.apiKeyNamePlaceholder")}
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-api-key-expiry">{t("system.apiKeyExpiry")}</Label>
            <select
              id="admin-api-key-expiry"
              data-testid="admin-api-key-expiry"
              className="border-input bg-background h-9 w-full rounded-md border px-2 text-sm"
              value={expiresIn}
              onChange={(event) => setExpiresIn(event.target.value)}
            >
              {EXPIRY_CHOICES.map((choice) => (
                <option key={choice.value} value={choice.value}>
                  {choice.label}
                </option>
              ))}
            </select>
          </div>
          <Button
            size="sm"
            data-testid="admin-api-key-create"
            disabled={creating || name.trim() === ""}
            onClick={() => void createKey()}
          >
            {t("system.apiKeyCreate")}
          </Button>
        </div>

        {freshSecret ? (
          <div className="space-y-1 rounded-md border p-3" data-testid="admin-api-key-secret">
            <p className="text-sm font-medium">
              {t("system.apiKeySecretTitle", { name: freshSecret.name })}
            </p>
            <code className="block break-all font-mono text-xs">{freshSecret.secret}</code>
            <div className="flex items-center justify-between gap-2">
              <p className="text-muted-foreground text-xs">{t("system.apiKeySecretHint")}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void navigator.clipboard.writeText(freshSecret.secret)}
              >
                {t("system.apiKeyCopy")}
              </Button>
            </div>
          </div>
        ) : null}

        <div className="space-y-1">
          {(keys ?? []).map((key) => (
            <div
              key={key.id}
              className="flex items-center justify-between gap-4 rounded-md border px-3 py-2"
            >
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">
                  {key.name}{" "}
                  <span className="text-muted-foreground font-mono text-xs">
                    {t("system.apiKeyPrefix", { prefix: key.prefix })}
                  </span>
                </p>
                <p className="text-muted-foreground text-xs">
                  {key.expires_at
                    ? t("system.apiKeyExpiresAt", { time: new Date(key.expires_at).toLocaleString() })
                    : t("system.apiKeyNoExpiry")}
                  {" · "}
                  {key.last_used_at
                    ? t("system.apiKeyLastUsed", { time: new Date(key.last_used_at).toLocaleString() })
                    : t("system.apiKeyNeverUsed")}
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                data-testid={`admin-api-key-revoke-${key.id}`}
                onClick={() => void revokeKey(key.id)}
              >
                {t("system.apiKeyRevoke")}
              </Button>
            </div>
          ))}
          {keys !== null && keys.length === 0 ? (
            <p className="text-muted-foreground text-xs">{t("system.apiKeyEmpty")}</p>
          ) : null}
        </div>
        <p className="text-muted-foreground text-xs">{t("system.apiKeyUsageHint")}</p>
      </CardContent>
    </Card>
  );
}
