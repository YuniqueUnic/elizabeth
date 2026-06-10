"use client";

import { useEffect, useState } from "react";
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
import { createRoom, getRoomDetails } from "@/api/roomService";
import { getAccessToken, hasValidToken, validateToken } from "@/api/authService";
import { clearRoomToken, getRoomTokenString } from "@/lib/utils/api";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { useRoomEvents, type RoomUpdatePayload } from "@/lib/hooks/use-room-events";
import { resolveWebSocketUrl } from "@/lib/utils/ws";
import { ContentType, parseContentType } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { useToast } from "@/hooks/use-toast";
import { useTranslations } from "next-intl";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
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
  const syncMessagesFromServer = useAppStore((state) =>
    state.syncMessagesFromServer
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
        void syncMessagesFromServer();
      }
    },
    onContentUpdated: (payload) => {
      notifyContentChange("updated", payload);
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text) {
        void syncMessagesFromServer();
      }
    },
    onContentDeleted: (payload) => {
      notifyContentChange("deleted", payload);
      void syncMessagesFromServer();
    },
    onRoomUpdate: (payload) => {
      notifyRoomUpdate(payload);
      onRoomUpdate?.(payload);
      queryClient.invalidateQueries({ queryKey: ["room", roomName] });
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

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [needsPassword, setNeedsPassword] = useState(false);
  const [tokenReady, setTokenReady] = useState(false);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const wsToken = tokenReady ? getRoomTokenString(roomName) : null;

  useEffect(() => {
    let isCancelled = false;

    const initRoom = async () => {
      if (!roomName) {
        router.push("/");
        return;
      }

      setLoading(true);
      setError(null);
      setNeedsPassword(false);
      setTokenReady(false);
      setRoomRedirectTarget(null);

      // This logic runs every time the `roomName` in the URL changes.
      // 1. Set the global room identifier.
      setCurrentRoomId(roomName);

      // 2. Check for a valid, non-expired token for this identifier.
      const hasToken = hasValidToken(roomName);
      console.log(`[RoomPage] Checking token for ${roomName}:`, hasToken);

      if (hasToken) {
        try {
          await validateToken(roomName);
          console.log(
            `[RoomPage] Valid token verified for ${roomName}, skipping authentication`,
          );
          if (!isCancelled) {
            setLoading(false);
            setTokenReady(true);
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

      console.log(
        `[RoomPage] No valid token for ${roomName}, initiating authentication`,
      );

      // 3. If no valid token, try to access the room to see if it's public,
      //    password-protected, or requires a special slug.
      try {
        const room = await getRoomDetails(roomName, undefined, true);
        if (isCancelled) return;

        // At this point, the room exists. Check for password.
        if (room.settings.passwordProtected && !hasValidToken(roomName)) {
          setNeedsPassword(true);
        } else if (!hasValidToken(roomName)) {
          // No password, so we should be able to get a token directly.
          await getAccessToken(roomName);
          if (!isCancelled) {
            setTokenReady(true);
          }
        }
      } catch (err: any) {
        if (isCancelled) return;

        const statusCandidate =
          err?.response?.status ?? err?.status ?? err?.code;
        const status = typeof statusCandidate === "number"
          ? statusCandidate
          : typeof statusCandidate === "string" &&
              Number.isFinite(Number(statusCandidate))
          ? Number(statusCandidate)
          : undefined;
        const rawMessage: string =
          typeof err?.message === "string" ? err.message : "";
        const message = rawMessage
          .replace(/^Validation error:\s*/i, "")
          .replace(/^Authentication failed:\s*/i, "");
        const isValidationError =
          status === 400 ||
          err?.code === "VALIDATION_ERROR" ||
          /^Validation error:/i.test(rawMessage);
        const isAuthenticationError =
          status === 401 ||
          status === 403 ||
          err?.code === "AUTHENTICATION_FAILED" ||
          /^Authentication failed:/i.test(rawMessage);

        if (isValidationError) {
          // Map backend validation errors to user-friendly Chinese messages
          const validationMessages: Record<string, string> = {
            "Room identifier cannot be empty":
              tErrors("enterRoomName"),
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
          setError(
            validationMessages[message] || message || tErrors("requestParameterError"),
          );
          return;
        }

        if (isAuthenticationError) {
          setError(
            tErrors("roomInaccessibleViaLink"),
          );
          return;
        }

        if (status === 404) {
          // Fallback: some deployments may not auto-create rooms on GET.
          // Try explicit create, then continue normal auth flow.
          try {
            await createRoom(roomName);
            const room = await getRoomDetails(roomName, undefined, true);

            if (room.settings.passwordProtected && !hasValidToken(roomName)) {
              setNeedsPassword(true);
              return;
            }

            await getAccessToken(roomName);
            setTokenReady(true);
            return;
          } catch (createErr: any) {
            const createStatus: number | undefined =
              createErr?.code ?? createErr?.status ?? createErr?.response?.status;
            if (createStatus === 409) {
              // Race: room created by someone else. Retry as existing room.
              try {
                const room = await getRoomDetails(roomName, undefined, true);
                if (room.settings.passwordProtected && !hasValidToken(roomName)) {
                  setNeedsPassword(true);
                  return;
                }
                await getAccessToken(roomName);
                setTokenReady(true);
                return;
              } catch {
                // Fall through to generic error
              }
            }
          }

          setError(tErrors("roomNotFoundAutoCreateFailed"));
          return;
        }

        setError(
          message ||
            tErrors("cannotAccessRoom"),
        );
      } finally {
        if (!isCancelled) {
          setLoading(false);
        }
      }
    };

    initRoom();

    return () => {
      isCancelled = true;
    };
  }, [roomName, setCurrentRoomId, router, setRoomRedirectTarget, tErrors]);

  const handlePasswordSubmit = async (password: string) => {
    try {
      setError(null);
      await getAccessToken(roomName, password);
      setNeedsPassword(false);
      setTokenReady(true);
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

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <LoadingSpinner className="h-12 w-12" />
          <p className="text-muted-foreground">{t("loadingRoom")}</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-background p-4">
        <Alert variant="destructive" className="max-w-md">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>{t("errorTitle")}</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      </div>
    );
  }

  if (needsPassword) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <RoomPasswordDialog
          roomName={roomName}
          open={needsPassword}
          onSubmit={handlePasswordSubmit}
          onCancel={handlePasswordCancel}
        />
      </div>
    );
  }

  // If we are not loading, have no errors, and don't need a password,
  // we can assume the room is accessible and render the main layout.
  // The token check in `initRoom` or a successful password submission ensures this.
  // Wait for tokenReady to be true before rendering child components.
  if (!tokenReady) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <LoadingSpinner className="h-12 w-12" />
          <p className="text-muted-foreground">{t("preparingRoomAccess")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      {tokenReady && wsToken && (
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
