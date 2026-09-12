"use client";

import { useCallback, useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { useQueryClient } from "@tanstack/react-query";
import { TopBar } from "@/components/layout/top-bar";
import { LeftSidebar } from "@/components/layout/left-sidebar";
import { MiddleColumn } from "@/components/layout/middle-column";
import { RightSidebar } from "@/components/layout/right-sidebar";
import { MobileLayout } from "@/components/layout/mobile-layout";
import { GlobalFilePreviewModal } from "@/components/files/global-file-preview-modal";
import { useIsMobile } from "@/hooks/use-mobile";
import { useAppStore } from "@/lib/store";
import { RoomPasswordDialog } from "@/components/room/room-password-dialog";
import { getAccessToken, hasValidToken, validateToken } from "@/api/authService";
import { clearRoomToken, getRoomTokenString } from "@/lib/utils/api";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { useRoomEvents, type RoomUpdatePayload } from "@/lib/hooks/use-room-events";
import { resolveWebSocketUrl } from "@/lib/utils/ws";
import { ContentType, parseContentType } from "@/lib/types";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useToast } from "@/hooks/use-toast";
import { useTranslations } from "next-intl";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { createRoom, getMyCapabilities, getRoomDetails } from "@/api/roomService";
import { setRoomToken, getRoomToken } from "@/lib/utils/api";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import { IdentityCodeDisclosure } from "@/components/room/identity-code-disclosure";
import {
  getContentNotificationKind,
  getContentNotificationSubject,
  isDesktopNotificationActionSupported,
  showContentDesktopNotification,
  showRoomDesktopNotification,
  type DesktopNotificationAction,
  type RoomDesktopNotificationAction,
} from "@/lib/desktop-notifications";
import type { ContentEventPayload } from "@/lib/hooks/use-room-events";

function roomUpdateNotificationAction(
  payload: RoomUpdatePayload,
  currentRoomName: string,
): RoomDesktopNotificationAction {
  const nextSlug = payload.room_info.slug.trim();
  if (nextSlug && nextSlug !== currentRoomName) {
    return "address_changed";
  }

  return payload.reason;
}

/**
 * 房间进入流程的显式状态机：loading → 校验本地凭据 →
 * （房间缺失）开通/（已有房间）门禁或访客进入 → ready/error。
 */
type RoomEntryPhase =
  | { kind: "loading" }
  | { kind: "gate"; passwordProtected: boolean }
  | { kind: "created"; identityCode: string }
  | { kind: "ready" }
  | { kind: "error"; message: string };

function responseStatusOf(error: unknown): number | undefined {
  const candidate: unknown =
    (error as { response?: { status?: unknown } })?.response?.status ??
    (error as { status?: unknown })?.status ??
    (error as { code?: unknown })?.code;
  if (typeof candidate === "number") return candidate;
  if (
    typeof candidate === "string" &&
    Number.isFinite(Number(candidate))
  ) {
    return Number(candidate);
  }
  return undefined;
}

function isAuthenticationFailure(error: unknown, status: number | undefined): boolean {
  const rawMessage: string =
    typeof (error as { message?: unknown })?.message === "string"
      ? (error as { message: string }).message
      : "";
  return (
    status === 401 ||
    status === 403 ||
    (error as { code?: string })?.code === "AUTHENTICATION_FAILED" ||
    /^Authentication failed:/i.test(rawMessage)
  );
}

function entryErrorMessage(
  error: unknown,
  status: number | undefined,
  tErrors: ReturnType<typeof useTranslations<"errors">>,
): string {
  const rawMessage: string =
    typeof (error as { message?: unknown })?.message === "string"
      ? (error as { message: string }).message
      : "";
  const message = rawMessage
    .replace(/^Validation error:\s*/i, "")
    .replace(/^Authentication failed:\s*/i, "");
  const isValidationError =
    status === 400 ||
    (error as { code?: string })?.code === "VALIDATION_ERROR" ||
    /^Validation error:/i.test(rawMessage);

  if (isValidationError) {
    // 后端校验文案 → 用户可读的本地化提示
    const validationMessages: Record<string, string> = {
      "Room identifier cannot be empty": tErrors("enterRoomName"),
      "Room identifier must be between 3 and 150 characters":
        tErrors("roomNameLength3to150"),
      "Room identifier can only contain letters, numbers, underscores, and hyphens":
        tErrors("backendRoomNameFormat"),
      "Room name must be between 3 and 50 characters":
        tErrors("backendRoomNameLength3to50"),
      "Room name can only contain letters, numbers, underscores, and hyphens, and cannot start or end with underscore or hyphen":
        tErrors("backendRoomNameFormat"),
      "Room password must be between 4 and 100 characters":
        tErrors("backendRoomPasswordLength4to100"),
    };
    return validationMessages[message] || message || tErrors("requestParameterError");
  }

  if (isAuthenticationFailure(error, status)) {
    return tErrors("roomInaccessibleViaLink");
  }
  if (status === 410) {
    return tErrors("roomExpired");
  }
  if (status === 404) {
    return tErrors("roomNotFound");
  }
  return message || tErrors("cannotAccessRoom");
}

function RoomRealtimeSync({
  roomName,
  token,
  onRoomUpdate,
}: {
  roomName: string;
  token: string;
  onRoomUpdate?: (payload: RoomUpdatePayload) => void;
}) {
  const t = useTranslations("common");
  const queryClient = useQueryClient();
  const desktopNotificationsEnabled = useAppStore((state) =>
    state.desktopNotificationsEnabled
  );
  const desktopNotificationTypes = useAppStore((state) =>
    state.desktopNotificationTypes
  );
  const desktopNotificationShowContent = useAppStore((state) =>
    state.desktopNotificationShowContent
  );
  const applyMessageCreated = useAppStore((state) => state.applyMessageCreated);
  const applyMessageUpdated = useAppStore((state) => state.applyMessageUpdated);
  const applyMessageDeleted = useAppStore((state) => state.applyMessageDeleted);
  const { has, payload } = useRoomCapabilities();
  const myJti = payload?.jti ?? null;
  const canManageMessageVisibility = has("msg.visibility.manage", "any")
    || has("msg.visibility.manage", "own");
  const bumpCapabilitiesVersion = useAppStore((state) => state.bumpCapabilitiesVersion);

  // 角色矩阵变更：拉取最新能力快照并更新本地 TokenInfo，UI 门禁随之实时生效
  const refreshMyCapabilities = useCallback(async () => {
    try {
      const current = getRoomToken(roomName);
      if (!current) return;
      const snapshot = await getMyCapabilities(roomName);
      setRoomToken(roomName, {
        ...current,
        capabilities: snapshot.capabilities,
        roleKey: snapshot.role,
      });
      bumpCapabilitiesVersion();
    } catch (error) {
      console.warn("Failed to refresh capabilities after roles change:", error);
    }
  }, [roomName, bumpCapabilitiesVersion]);
  const refreshLatestMessages = useAppStore((state) =>
    state.refreshLatestMessages
  );

  const notifyContentChange = (
    action: DesktopNotificationAction,
    payload: ContentEventPayload,
  ) => {
    const kind = getContentNotificationKind(payload);
    if (!kind) return;
    if (!isDesktopNotificationActionSupported(kind, action)) return;

    const subject = getContentNotificationSubject(payload, kind) ||
      t(`desktopNotification.fallback.${kind}`);
    const summary = t(`desktopNotification.summary.${kind}.${action}`);

    showContentDesktopNotification({
      enabled: desktopNotificationsEnabled,
      types: desktopNotificationTypes,
      payload,
      action,
      roomName,
      title: t(`desktopNotification.title.${kind}.${action}`),
      body: desktopNotificationShowContent
        ? t("desktopNotification.bodyWithSubject", { roomName, subject })
        : t("desktopNotification.bodyWithoutSubject", { roomName, summary }),
    });
  };

  const notifyRoomUpdate = (payload: RoomUpdatePayload) => {
    const action = roomUpdateNotificationAction(payload, roomName);
    const nextSlug = payload.room_info.slug.trim();
    const addressPath = `/${nextSlug || payload.room_name || roomName}`;
    const subject = action === "address_changed"
      ? t("desktopNotification.roomUpdateSubject.addressChanged", {
        path: addressPath,
      })
      : t(`desktopNotification.roomUpdateSubject.${action}`);
    const summary = t(`desktopNotification.summary.room.${action}`);

    showRoomDesktopNotification({
      enabled: desktopNotificationsEnabled,
      types: desktopNotificationTypes,
      action,
      roomName,
      title: t(`desktopNotification.title.room.${action}`),
      body: desktopNotificationShowContent
        ? t("desktopNotification.bodyWithSubject", { roomName, subject })
        : t("desktopNotification.bodyWithoutSubject", { roomName, summary }),
      tagSubject: nextSlug || payload.room_name || roomName,
    });
  };

  useRoomEvents({
    wsUrl: resolveWebSocketUrl(),
    roomName,
    token,
    enableCacheInvalidation: true,
    onContentCreated: (payload) => {
      notifyContentChange("created", payload);
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text) {
        applyMessageCreated(payload);
      }
    },
    onContentUpdated: (payload) => {
      // 隐藏事件不带正文；无管理能力的成员直接从本地缓存移除，
      // 持有 msg.visibility.manage 的成员就地合并 hidden 状态
      if (payload.hidden && payload.content_id != null
        && payload.created_by_jti !== myJti
        && !canManageMessageVisibility) {
        applyMessageDeleted(String(payload.content_id));
        return;
      }
      notifyContentChange("updated", payload);
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text) {
        applyMessageUpdated(payload);
      }
    },
    onContentDeleted: (payload) => {
      notifyContentChange("deleted", payload);
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text && payload.content_id != null) {
        applyMessageDeleted(String(payload.content_id));
      }
    },
    onReconnected: () => {
      void refreshLatestMessages().catch((error) => {
        console.error("Failed to refresh messages after reconnect:", error);
      });
    },
    onRoomUpdate: (payload) => {
      notifyRoomUpdate(payload);
      onRoomUpdate?.(payload);
      queryClient.invalidateQueries({ queryKey: ["room", roomName] });
      if (payload.reason === "roles_changed") {
        void refreshMyCapabilities();
      }
    },
  });

  return null;
}

export default function RoomPage() {
  const t = useTranslations("common");
  const tErrors = useTranslations("errors");
  const pathname = usePathname();
  const router = useRouter();
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { currentRoomId, setCurrentRoomId } = useAppStore();
  const roomRedirectTarget = useAppStore((state) => state.roomRedirectTarget);
  const setRoomRedirectTarget = useAppStore((state) =>
    state.setRoomRedirectTarget
  );
  const hasUnsavedChanges = useAppStore((state) => state.hasUnsavedChanges);

  // 始终从浏览器真实 URL 解析房间名，解决 Next.js 静态导出时
  // useParams() 返回编译期占位值（而非真实路径）的水合冲突问题
  const roomName = pathname.split("/").filter(Boolean)[0] ?? "";

  const [entry, setEntry] = useState<RoomEntryPhase>({ kind: "loading" });
  const [manualCopyValue, setManualCopyValue] = useState("");
  const wsToken = entry.kind === "ready" ? getRoomTokenString(roomName) : null;

  useEffect(() => {
    let isCancelled = false;

    const enterExistingRoom = async () => {
      try {
        const room = await getRoomDetails(roomName, undefined, true);
        if (isCancelled) return;

        if (room.settings.passwordProtected) {
          setEntry({ kind: "gate", passwordProtected: true });
          return;
        }
        await getAccessToken(roomName);
        if (!isCancelled) {
          setEntry({ kind: "ready" });
        }
      } catch (err: unknown) {
        if (isCancelled) return;
        const status = responseStatusOf(err);
        if (status === 404) {
          await provisionDirectUrlRoom();
          return;
        }
        setEntry({ kind: "error", message: entryErrorMessage(err, status, tErrors) });
      }
    };

    // URL 直达一个不存在的合法房间名即零步开通：访问本身就是创建命令。
    // 与首页创建共用同一条 POST 命令，创建者由此获得 admin 会话与
    // 一次性 admin 身份码，而不是被静默降级为默认读者角色。
    const provisionDirectUrlRoom = async () => {
      try {
        const created = await createRoom(roomName);
        if (isCancelled) return;
        setRoomToken(roomName, {
          token: created.token,
          expiresAt: created.expires_at,
          capabilities: created.capabilities,
          roleKey: created.claims.role,
        });
        if (!created.identity_code) {
          setEntry({ kind: "ready" });
          return;
        }
        setEntry({ kind: "created", identityCode: created.identity_code });
      } catch (err: unknown) {
        if (isCancelled) return;
        const status = responseStatusOf(err);
        if (status === 409) {
          // 并发访问时房间已由先到者创建；回退为普通进入流程。
          await enterExistingRoom();
          return;
        }
        setEntry({ kind: "error", message: entryErrorMessage(err, status, tErrors) });
      }
    };

    const initRoom = async () => {
      if (!roomName) {
        router.push("/");
        return;
      }

      setEntry({ kind: "loading" });
      setRoomRedirectTarget(null);
      setCurrentRoomId(roomName);

      // 1. 校验本地已保存的、未过期的凭据。
      if (hasValidToken(roomName)) {
        try {
          await validateToken(roomName);
          if (!isCancelled) {
            setEntry({ kind: "ready" });
          }
          return;
        } catch (err) {
          console.warn(
            `[RoomPage] Stored token rejected by backend for ${roomName}, clearing it`,
            err,
          );
          clearRoomToken(roomName);
        }
      }

      // 2. 无本地凭据：查询房间并按状态进入。
      await enterExistingRoom();
    };

    initRoom();

    return () => {
      isCancelled = true;
    };
  }, [roomName, setCurrentRoomId, router, setRoomRedirectTarget, tErrors]);

  const handlePasswordSubmit = async (password: string) => {
    try {
      await getAccessToken(roomName, password);
      setEntry({ kind: "ready" });
    } catch (err: any) {
      console.error("Password submission failed:", err);
      if (
        err.code === 401 ||
        err.message?.toLowerCase().includes("password") ||
        err.message?.toLowerCase().includes("authentication")
      ) {
        throw new Error(tErrors("wrongPassword"));
      } else {
        throw new Error(tErrors("passwordVerificationRetry"));
      }
    }
  };

  const handlePasswordCancel = () => {
    router.push("/");
  };

  if (entry.kind === "loading") {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <LoadingSpinner className="h-12 w-12" />
          <p className="text-muted-foreground">{t("loadingRoom")}</p>
        </div>
      </div>
    );
  }

  if (entry.kind === "error") {
    return (
      <div className="flex h-screen items-center justify-center bg-background p-4">
        <Alert variant="destructive" className="max-w-md">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>{t("errorTitle")}</AlertTitle>
          <AlertDescription>{entry.message}</AlertDescription>
        </Alert>
      </div>
    );
  }

  if (entry.kind === "gate") {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <RoomPasswordDialog
          roomName={roomName}
          open
          onSubmit={handlePasswordSubmit}
          onCancel={handlePasswordCancel}
        />
      </div>
    );
  }

  if (entry.kind === "created") {
    return (
      <div className="flex h-screen items-center justify-center bg-background p-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>{t("roomReadyTitle")}</CardTitle>
            <CardDescription>{t("roomReadyDescription", { roomName })}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <IdentityCodeDisclosure
              code={entry.identityCode}
              onEnter={() => setEntry({ kind: "ready" })}
            />
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      {wsToken && (
        <RoomRealtimeSync
          roomName={roomName}
          token={wsToken}
          onRoomUpdate={(payload) => {
            const nextSlug = payload?.room_info?.slug;
            if (typeof nextSlug !== "string" || !nextSlug.trim()) return;

            if (nextSlug !== roomName) {
              clearRoomToken(roomName);
              setRoomRedirectTarget(nextSlug);
              toast({
                title: t("roomAddressChanged"),
                description: t("roomRedirectedToast"),
              });
            }
          }}
        />
      )}
      {roomRedirectTarget && (
        <div className="p-3">
          <Alert>
            <AlertTitle>{t("roomAddressChanged")}</AlertTitle>
            <AlertDescription className="space-y-2">
              <p className="text-sm text-muted-foreground">
                {t("roomRedirectDescription", { path: `/${roomRedirectTarget}` })}
              </p>
              {hasUnsavedChanges() && (
                <p className="text-sm font-medium text-destructive">
                  {t("unsavedChangesWarning")}
                </p>
              )}
              <div className="flex flex-wrap gap-2">
                <Button
                  size="sm"
                  onClick={() => {
                    router.push(`/${roomRedirectTarget}`);
                  }}
                >
                  {t("goToNewAddress")}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={async () => {
                    const value = `${window.location.origin}/${roomRedirectTarget}`;
                    try {
                      await copyTextToClipboard(value);
                      toast({ title: t("copiedNewLink") });
                    } catch (err) {
                      console.error("Failed to copy link:", err);
                      setManualCopyValue(value);
                      toast({
                        title: t("copyFailed"),
                        description: t("copyLinkFailed"),
                        variant: "destructive",
                      });
                    }
                  }}
                >
                  {t("copyNewLink")}
                </Button>
              </div>
            </AlertDescription>
          </Alert>
        </div>
      )}
      <TopBar />
      {isMobile
        ? (
          <div className="flex-1 overflow-hidden">
            <MobileLayout />
          </div>
        )
        : (
          <div className="flex min-w-0 flex-1 overflow-hidden">
            <LeftSidebar />
            <MiddleColumn />
            <RightSidebar />
          </div>
        )}

      {/* Always mounted — handles file preview triggered from message bubbles */}
      <GlobalFilePreviewModal />
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setManualCopyValue("");
        }}
      />
    </div>
  );
}
