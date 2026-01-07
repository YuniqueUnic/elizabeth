"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQueryClient } from "@tanstack/react-query";
import { TopBar } from "@/components/layout/top-bar";
import { LeftSidebar } from "@/components/layout/left-sidebar";
import { MiddleColumn } from "@/components/layout/middle-column";
import { RightSidebar } from "@/components/layout/right-sidebar";
import { MobileLayout } from "@/components/layout/mobile-layout";
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

function RoomRealtimeSync({
  roomName,
  token,
  onRoomUpdate,
}: {
  roomName: string;
  token: string;
  onRoomUpdate?: (payload: RoomUpdatePayload) => void;
}) {
  const queryClient = useQueryClient();
  const syncMessagesFromServer = useAppStore((state) =>
    state.syncMessagesFromServer
  );

  useRoomEvents({
    wsUrl: resolveWebSocketUrl(),
    roomName,
    token,
    enableCacheInvalidation: true,
    onContentCreated: (payload) => {
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text) {
        void syncMessagesFromServer();
      }
    },
    onContentUpdated: (payload) => {
      const kind = parseContentType(payload.content_type);
      if (kind === ContentType.Text) {
        void syncMessagesFromServer();
      }
    },
    onContentDeleted: () => {
      // Deleted payload does not include content_type; refresh messages to stay consistent.
      void syncMessagesFromServer();
    },
    onRoomUpdate: (payload) => {
      onRoomUpdate?.(payload);
      queryClient.invalidateQueries({ queryKey: ["room", roomName] });
    },
  });

  return null;
}

export default function RoomPage() {
  const params = useParams();
  const router = useRouter();
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { currentRoomId, setCurrentRoomId } = useAppStore();

  const roomName = params.roomName as string;

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [needsPassword, setNeedsPassword] = useState(false);
  const [tokenReady, setTokenReady] = useState(false);
  const [roomRedirect, setRoomRedirect] = useState<string | null>(null);
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
      setRoomRedirect(null);

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

        const status: number | undefined =
          err?.code ?? err?.status ?? err?.response?.status;
        const message: string =
          typeof err?.message === "string" ? err.message : "";

        if (status === 400) {
          setError(
            "房间名称不合法：仅支持 3-50 位字母/数字/下划线/连字符，且不能以下划线或连字符开头/结尾。",
          );
          return;
        }

        if (status === 401 || status === 403) {
          setError(
            "房间无法通过该链接进入：可能已过期、达到最大进入次数，或已切换为私密地址（请使用新的分享链接）。",
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

          setError("房间不存在，且自动创建失败。请稍后重试。");
          return;
        }

        setError(
          message ||
            "无法访问房间，请稍后重试（请检查后端服务是否可用以及前端 API 代理配置）。",
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
  }, [roomName, setCurrentRoomId, router]);

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
        throw new Error("密码错误");
      } else {
        throw new Error("无法验证密码，请稍后重试");
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
          <p className="text-muted-foreground">正在加载房间...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-background p-4">
        <Alert variant="destructive" className="max-w-md">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>错误</AlertTitle>
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
          <p className="text-muted-foreground">正在准备房间访问...</p>
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
              setRoomRedirect(nextSlug);
              toast({
                title: "房间地址已变更",
                description: "该房间已切换到新的地址，请跳转继续。",
              });
            }
          }}
        />
      )}
      {roomRedirect && (
        <div className="p-3">
          <Alert>
            <AlertTitle>房间地址已变更</AlertTitle>
            <AlertDescription className="space-y-2">
              <p className="text-sm text-muted-foreground">
                该房间已切换到新的地址：<span className="font-mono">/{roomRedirect}</span>。
                为继续使用，请跳转到新地址并重新登录。
              </p>
              <div className="flex flex-wrap gap-2">
                <Button
                  size="sm"
                  onClick={() => {
                    router.push(`/${roomRedirect}`);
                  }}
                >
                  跳转到新地址
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(
                        `${window.location.origin}/${roomRedirect}`,
                      );
                      toast({ title: "已复制新链接" });
                    } catch (err) {
                      console.error("Failed to copy link:", err);
                      toast({
                        title: "复制失败",
                        description: "无法复制链接，请手动复制。",
                        variant: "destructive",
                      });
                    }
                  }}
                >
                  复制新链接
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
          <div className="flex flex-1 overflow-hidden">
            <LeftSidebar />
            <MiddleColumn />
            <RightSidebar />
          </div>
        )}
    </div>
  );
}
