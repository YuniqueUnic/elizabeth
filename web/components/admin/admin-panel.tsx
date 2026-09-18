"use client";

import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";

import {
  adminLogin,
  adminMe,
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

const ADMIN_SESSION_STORAGE_KEY = "elizabeth.admin-session";

type AdminPanelState =
  | { status: "login" }
  | { status: "loading" }
  | { status: "ready"; sessionToken: string };

type Tab = "dashboard" | "rooms" | "system";

/**
 * 管理面板容器：登录态、tab 导航与数据加载。
 * 会话令牌只保存在 sessionStorage；真正的鉴权始终发生在服务端。
 */
export function AdminPanel() {
  const t = useTranslations("admin");
  const { toast } = useToast();
  const [state, setState] = useState<AdminPanelState>({ status: "login" });
  const [tab, setTab] = useState<Tab>("dashboard");
  const [stats, setStats] = useState<AdminStatsResponse | null>(null);
  const [storage, setStorage] = useState<AdminStorageResponse | null>(null);
  const [config, setConfig] = useState<AdminConfigResponse | null>(null);
  // 递增触发房间管理列表重新加载（刷新按钮 / 删除后）
  const [roomsRefreshKey, setRoomsRefreshKey] = useState(0);

  const reportError = useCallback(
    (message: string) => {
      toast({ description: t("errors.loadFailed", { message }), variant: "destructive" });
    },
    [toast, t],
  );

  const loadSystem = useCallback(
    async (sessionToken: string) => {
      const [nextStorage, nextConfig] = await Promise.all([
        getAdminStorage(sessionToken),
        getAdminConfig(sessionToken),
      ]);
      setStorage(nextStorage);
      setConfig(nextConfig);
    },
    [],
  );

  const refreshStats = useCallback(
    async (sessionToken: string) => {
      setStats(await getAdminStats(sessionToken));
    },
    [],
  );

  useEffect(() => {
    const saved = sessionStorage.getItem(ADMIN_SESSION_STORAGE_KEY);
    if (!saved) return;
    setState({ status: "loading" });
    adminMe(saved)
      .then(async (session) => {
        setStats(await getAdminStats(saved));
        await loadSystem(saved);
        setState({ status: "ready", sessionToken: saved });
      })
      .catch(() => {
        sessionStorage.removeItem(ADMIN_SESSION_STORAGE_KEY);
        setState({ status: "login" });
      });
  }, [loadSystem]);

  async function handleLogin(username: string, password: string) {
    setState({ status: "loading" });
    try {
      const session = await adminLogin(username, password);
      setStats(await getAdminStats(session.token));
      await loadSystem(session.token);
      sessionStorage.setItem(ADMIN_SESSION_STORAGE_KEY, session.token);
      setState({ status: "ready", sessionToken: session.token });
    } catch {
      toast({
        description: t("login.invalid"),
        variant: "destructive",
      });
      setState({ status: "login" });
    }
  }

  function handleLogout() {
    sessionStorage.removeItem(ADMIN_SESSION_STORAGE_KEY);
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

  const sessionToken = state.sessionToken;

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
              if (tab === "rooms") {
                setRoomsRefreshKey((key) => key + 1);
                return;
              }
              refreshStats(sessionToken).catch(reportError);
              loadSystem(sessionToken).catch(reportError);
            }}
          >
            {t("nav.refresh")}
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
          sessionToken={sessionToken}
          refreshKey={roomsRefreshKey}
          onError={reportError}
          onDeleted={(name) => {
            toast({ description: t("rooms.deleted", { name }) });
            refreshStats(sessionToken).catch(reportError);
          }}
          onSaved={() => {
            toast({ description: t("rooms.saved") });
          }}
        />
      ) : null}
      {tab === "system" && storage && config ? (
        <AdminSystem
          storage={storage}
          config={config}
          sessionToken={sessionToken}
          onError={reportError}
          onSaved={(updated) => {
            setConfig(updated);
            toast({ description: t("system.saved") });
          }}
          onPasswordChanged={() => {
            toast({ description: t("system.passwordChanged") });
            // 改密使当前会话一并失效，回到登录页
            handleLogout();
          }}
        />
      ) : null}
    </div>
  );
}
