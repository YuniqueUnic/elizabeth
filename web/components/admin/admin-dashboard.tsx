"use client";

import { useTranslations } from "next-intl";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatFileSize } from "@/lib/utils/format";
import type { AdminStatsResponse } from "@/types/generated/api.types";

function StatCard({ label, value }: { label: string; value: string | number }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-muted-foreground text-sm font-medium">{label}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-semibold tabular-nums">{value}</div>
      </CardContent>
    </Card>
  );
}

/** 概览（展示组件）：纯渲染服务端聚合数据。 */
export function AdminDashboard({ stats }: { stats: AdminStatsResponse }) {
  const t = useTranslations("admin");

  return (
    <div className="space-y-6">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label={t("stats.roomsTotal")} value={stats.rooms_total} />
        <StatCard label={t("stats.roomsOpen")} value={stats.rooms_open} />
        <StatCard label={t("stats.roomsProtected")} value={stats.rooms_protected} />
        <StatCard
          label={t("stats.activeConnections")}
          value={stats.active_connections}
        />
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label={t("stats.contentsTotal")} value={stats.contents_total} />
        <StatCard label={t("stats.contentsFiles")} value={stats.contents_files} />
        <StatCard
          label={t("stats.contentsMessages")}
          value={stats.contents_messages}
        />
        <StatCard label={t("stats.activeRooms")} value={stats.active_rooms} />
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          label={t("stats.logicalBytes")}
          value={formatFileSize(stats.storage_logical_bytes)}
        />
        <StatCard
          label={t("stats.physicalBytes")}
          value={formatFileSize(stats.storage_physical_bytes)}
        />
        <StatCard label={t("stats.blobCount")} value={stats.blob_count} />
        <StatCard
          label={t("stats.dedupSaved")}
          value={formatFileSize(
            Math.max(stats.storage_logical_bytes - stats.storage_physical_bytes, 0),
          )}
        />
      </div>
    </div>
  );
}
