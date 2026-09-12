"use client";

import { useTranslations } from "next-intl";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatFileSize } from "@/lib/utils/format";
import type {
  AdminConfigResponse,
  AdminStorageResponse,
} from "@/types/generated/api.types";

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-1.5">
      <span className="text-muted-foreground text-sm">{label}</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  );
}

/** 存储 + 系统配置（展示组件）：只读渲染，机密由服务端过滤。 */
export function AdminSystem({
  storage,
  config,
}: {
  storage: AdminStorageResponse;
  config: AdminConfigResponse;
}) {
  const t = useTranslations("admin");

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
            <Row
              label={t("system.presignBase")}
              value={storage.presign_base_url}
            />
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
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("system.configTitle")}</CardTitle>
        </CardHeader>
        <CardContent className="divide-y">
          <Row label={t("system.server")} value={`${config.server_host}:${config.server_port}`} />
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
            label={t("system.jwtTtl")}
            value={`${config.jwt_ttl_seconds}s`}
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
          <p className="text-muted-foreground pt-2 text-xs">
            {t("system.readonlyHint")}
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
