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
  const { setCurrentRoomId } = useAppStore();

  const roomName = params.roomName as string;

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [needsPassword, setNeedsPassword] = useState(false);
  const [roomExists, setRoomExists] = useState(false);

  useEffect(() => {
    if (!roomName) {
      router.push("/");
      return;
    }

    // Set current room ID
    setCurrentRoomId(roomName);

    // Check if we already have a valid token in localStorage
    const checkTokenAndAccess = () => {
      try {
        const stored = localStorage.getItem("elizabeth_tokens");

        if (stored) {
          const tokens = JSON.parse(stored);
          const tokenInfo = tokens[roomName];

          if (tokenInfo && tokenInfo.expiresAt) {
            const isValid = !isTokenExpired(tokenInfo.expiresAt, 0);

            if (isValid) {
              setLoading(false);
              setRoomExists(true);
              return;
            } else {
              delete tokens[roomName];
              localStorage.setItem("elizabeth_tokens", JSON.stringify(tokens));
            }
          }
        }
      } catch (e) {
        console.error("Error reading tokens:", e);
      }

      checkRoomAccess();
    };

    checkTokenAndAccess();
  }, [roomName, router, setCurrentRoomId]);

  const checkRoomAccess = async () => {
    try {
      setLoading(true);
      setError(null);

      // Try to get room details without token (skipAuth=true)
      const room = await getRoomDetails(roomName, undefined, true);

      // Check if room has SHARE permission
      // If not shareable, verify that roomName matches the slug (UUID link)
      const isShareable = room.permissions.includes("share");
      const roomSlug = room.slug || room.name;

      // If room is not shareable and name doesn't match slug, reject direct access
      if (!isShareable && roomName !== roomSlug && roomSlug.includes("_")) {
        setError(
          "该房间不允许直接访问，请使用管理员提供的完整链接（包含 UUID）",
        );
        setLoading(false);
        return;
      }

      // Room exists and is accessible
      setRoomExists(true);

      // If room has password, show password dialog
      if (room.settings.passwordProtected) {
        setNeedsPassword(true);
        setLoading(false);
      } else {
        // No password needed, try to get token
        try {
          await getAccessToken(roomName);
          setLoading(false);
        } catch (err) {
          setError("无法获取访问令牌");
          setLoading(false);
        }
      }
    } catch (err: any) {
      // Room doesn't exist or other error
      if (err.message?.includes("404") || err.message?.includes("not found")) {
        setError("房间不存在");
      } else {
        setError("无法访问房间，请稍后重试");
      }
      setLoading(false);
    }
  };

  const handlePasswordSubmit = async (password: string) => {
    try {
      setError(null);
      await getAccessToken(roomName, password);
      setNeedsPassword(false);
      setRoomExists(true);
    } catch (err: any) {
      console.error("Password submission failed:", err);
      // Check for authentication errors (401) or password-related errors
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

  if (!roomExists) {
    return (
      <div className="flex h-screen items-center justify-center bg-background p-4">
        <Alert className="max-w-md">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>房间不存在</AlertTitle>
          <AlertDescription>
            您访问的房间不存在或已被删除。
          </AlertDescription>
        </Alert>
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
