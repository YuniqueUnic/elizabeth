"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { TopBar } from "@/components/layout/top-bar";
import { LeftSidebar } from "@/components/layout/left-sidebar";
import { MiddleColumn } from "@/components/layout/middle-column";
import { RightSidebar } from "@/components/layout/right-sidebar";
import { MobileLayout } from "@/components/layout/mobile-layout";
import { useIsMobile } from "@/hooks/use-mobile";
import { useAppStore } from "@/lib/store";
import { RoomPasswordDialog } from "@/components/room/room-password-dialog";
import { getRoomDetails } from "@/api/roomService";
import { getAccessToken, hasValidToken } from "@/api/authService";
import { isTokenExpired } from "@/lib/utils/api";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";

export default function RoomPage() {
  const params = useParams();
  const router = useRouter();
  const isMobile = useIsMobile();
  const { currentRoomId, setCurrentRoomId } = useAppStore();

  const roomName = params.roomName as string;

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [needsPassword, setNeedsPassword] = useState(false);
  const [tokenReady, setTokenReady] = useState(false);

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

      // This logic runs every time the `roomName` in the URL changes.
      // 1. Set the global room identifier.
      setCurrentRoomId(roomName);

      // 2. Check for a valid, non-expired token for this identifier.
      const hasToken = hasValidToken(roomName);
      console.log(`[RoomPage] Checking token for ${roomName}:`, hasToken);

      if (hasToken) {
        console.log(
          `[RoomPage] Valid token found for ${roomName}, skipping authentication`,
        );
        if (!isCancelled) {
          setLoading(false);
          setTokenReady(true);
        }
        return;
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
        if (!isCancelled) {
          if (err.message?.includes("404")) {
            setError("房间不存在");
          } else {
            setError("无法访问房间，请稍后重试");
          }
        }
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
