"use client";

import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";

import {
  getAdminConfig,
  getAdminStats,
  getAdminStorage,
} from "@/api/adminService";
import { AdminDashboard } from "@/components/admin/admin-dashboard";
import { AdminLogin } from "@/components/admin/admin-login";
import { AdminRooms } from "@/components/admin/admin-rooms";
import { AdminSystem } from "@/components/admin/admin-system";
import { Button } from "@/components/ui/button";
import { useToast } from "@/hooks/use-toast";
import type {
  AdminConfigResponse,
  AdminStatsResponse,
  AdminStorageResponse,
} from "@/types/generated/api.types";

const ADMIN_TOKEN_STORAGE_KEY = "elizabeth.admin-token";

type AdminPanelState =
  | { status: "login" }
  | { status: "loading" }
  | { status: "ready"; adminToken: string };

type Tab = "dashboard" | "rooms" | "system";

/**
 * 管理面板容器：登录态、tab 导航与数据加载。
 * token 只保存在 sessionStorage；真正的鉴权始终发生在服务端。
 */
export function AdminPanel() {
  const t = useTranslations("admin");
  const { toast } = useToast();
  const [state, setState] = useState<AdminPanelState>({ status: "login" });
  const [tab, setTab] = useState<Tab>("dashboard");
  const [stats, setStats] = useState<AdminStatsResponse | null>(null);
  const [storage, setStorage] = useState<AdminStorageResponse | null>(null);
  const [config, setConfig] = useState<AdminConfigResponse | null>(null);

  const reportError = useCallback(
    (message: string) => {
      toast({ description: t("errors.loadFailed", { message }), variant: "destructive" });
    },
    [toast, t],
  );

  const loadSystem = useCallback(
    async (adminToken: string) => {
      const [nextStorage, nextConfig] = await Promise.all([
        getAdminStorage(adminToken),
        getAdminConfig(adminToken),
      ]);
      setStorage(nextStorage);
      setConfig(nextConfig);
    },
    [],
  );

  const refreshStats = useCallback(
    async (adminToken: string) => {
      setStats(await getAdminStats(adminToken));
    },
    [],
  );

  useEffect(() => {
    const saved = sessionStorage.getItem(ADMIN_TOKEN_STORAGE_KEY);
    if (!saved) return;
    setState({ status: "loading" });
    getAdminStats(saved)
      .then(async (result) => {
        setStats(result);
        await loadSystem(saved);
        setState({ status: "ready", adminToken: saved });
      })
      .catch(() => {
        sessionStorage.removeItem(ADMIN_TOKEN_STORAGE_KEY);
        setState({ status: "login" });
      });
  }, [loadSystem]);

  function handleLogin(adminToken: string) {
    setState({ status: "loading" });
    getAdminStats(adminToken)
      .then(async (result) => {
        setStats(result);
        await loadSystem(adminToken);
        sessionStorage.setItem(ADMIN_TOKEN_STORAGE_KEY, adminToken);
        setState({ status: "ready", adminToken });
      })
      .catch(() => {
        toast({
          description: t("login.invalid"),
          variant: "destructive",
        });
        setState({ status: "login" });
      });
  }

  function handleLogout() {
    sessionStorage.removeItem(ADMIN_TOKEN_STORAGE_KEY);
    setStats(null);
    setStorage(null);
    setConfig(null);
    setState({ status: "login" });
  }

  if (state.status !== "ready") {
    return (
      <div className="space-y-4">
        {state.status === "loading" ? (
          <p className="text-muted-foreground text-center text-sm">...</p>
        ) : null}
        <AdminLogin onSubmit={handleLogin} />
      </div>
    );
  }

  const adminToken = state.adminToken;

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <nav className="flex gap-2">
          {(
            [
              ["dashboard", t("nav.dashboard")],
              ["rooms", t("nav.rooms")],
              ["system", t("nav.system")],
            ] as const
          ).map(([key, label]) => (
            <Button
              key={key}
              variant={tab === key ? "default" : "ghost"}
              size="sm"
              onClick={() => setTab(key)}
            >
              {label}
            </Button>
          ))}
        </nav>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refreshStats(adminToken).catch(reportError);
              loadSystem(adminToken).catch(reportError);
            }}
          >
            {t("nav.dashboard")}
          </Button>
          <Button variant="ghost" size="sm" onClick={handleLogout}>
            {t("nav.logout")}
          </Button>
        </div>
      </header>

      {tab === "dashboard" && stats ? (
        <AdminDashboard stats={stats} />
      ) : null}
      {tab === "rooms" ? (
        <AdminRooms
          adminToken={adminToken}
          onError={reportError}
          onDeleted={(name) => {
            toast({ description: t("rooms.deleted", { name }) });
            refreshStats(adminToken).catch(reportError);
          }}
        />
      ) : null}
      {tab === "system" && storage && config ? (
        <AdminSystem storage={storage} config={config} />
      ) : null}
    </div>
  );
}
