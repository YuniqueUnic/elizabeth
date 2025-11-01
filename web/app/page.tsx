"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { createRoom } from "@/api/roomService";
import { getAccessToken } from "@/api/authService";
import { ArrowRight, Lock, Plus } from "lucide-react";
import { ThemeSwitcher } from "@/components/theme-switcher";

export default function HomePage() {
  const router = useRouter();
  const [mode, setMode] = useState<"home" | "create" | "join">("home");
  const [roomName, setRoomName] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreateRoom = async () => {
    if (!roomName.trim()) {
      setError("请输入房间名称");
      return;
    }

    try {
      setLoading(true);
      setError(null);

      // Create room with optional password
      await createRoom(roomName, password || undefined);

      // Get access token
      await getAccessToken(roomName, password || undefined);

      // Navigate to room
      router.push(`/${roomName}`);
    } catch (err: any) {
      if (err.message?.includes("409") || err.message?.includes("exists")) {
        setError("房间名称已存在，请使用其他名称");
      } else {
        setError(err.message || "创建房间失败，请重试");
      }
    } finally {
      setLoading(false);
    }
  };

  const handleJoinRoom = () => {
    if (!roomName.trim()) {
      setError("请输入房间名称");
      return;
    }

    // Navigate to room page, it will handle password if needed
    router.push(`/${roomName}`);
  };

  if (mode === "home") {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center bg-linear-to-br from-background to-muted/20 p-4">
        <div className="absolute top-4 right-4">
          <ThemeSwitcher />
        </div>

        <div className="w-full max-w-md space-y-8">
          <div className="text-center">
            <h1 className="text-4xl font-bold tracking-tight">Elizabeth</h1>
            <p className="mt-2 text-muted-foreground">
              安全、临时、可控的文件分享与协作平台
            </p>
          </div>

          <div className="space-y-4">
            <Card
              className="cursor-pointer transition-all hover:shadow-lg"
              onClick={() => setMode("create")}
            >
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
                    <Plus className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <CardTitle>创建房间</CardTitle>
                    <CardDescription>创建一个新的协作空间</CardDescription>
                  </div>
                </div>
              </CardHeader>
            </Card>

            <Card
              className="cursor-pointer transition-all hover:shadow-lg"
              onClick={() => setMode("join")}
            >
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
                    <ArrowRight className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <CardTitle>加入房间</CardTitle>
                    <CardDescription>通过房间名称加入现有房间</CardDescription>
                  </div>
                </div>
              </CardHeader>
            </Card>
          </div>

          <div className="text-center text-sm text-muted-foreground">
            <p>无需注册，房间即身份</p>
          </div>
        </div>
      </div>
    );
  }

  if (mode === "create") {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center bg-linear-to-br from-background to-muted/20 p-4">
        <div className="absolute top-4 right-4">
          <ThemeSwitcher />
        </div>

        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>创建房间</CardTitle>
            <CardDescription>
              设置房间名称和可选的密码保护
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="room-name">房间名称 *</Label>
              <Input
                id="room-name"
                value={roomName}
                onChange={(e) => {
                  setRoomName(e.target.value);
                  setError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && roomName.trim() && !loading) {
                    handleCreateRoom();
                  }
                }}
                placeholder="例如：project-alpha"
                disabled={loading}
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">
                <div className="flex items-center gap-2">
                  <Lock className="h-4 w-4" />
                  <span>密码保护（可选）</span>
                </div>
              </Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && roomName.trim() && !loading) {
                    handleCreateRoom();
                  }
                }}
                placeholder="留空表示不设置密码"
                disabled={loading}
              />
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="flex gap-2">
              <Button
                variant="outline"
                onClick={() => {
                  setMode("home");
                  setRoomName("");
                  setPassword("");
                  setError(null);
                }}
                disabled={loading}
                className="flex-1"
              >
                返回
              </Button>
              <Button
                onClick={handleCreateRoom}
                disabled={loading || !roomName.trim()}
                className="flex-1"
              >
                {loading ? "创建中..." : "创建房间"}
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Join mode
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-linear-to-br from-background to-muted/20 p-4">
      <div className="absolute top-4 right-4">
        <ThemeSwitcher />
      </div>

      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>加入房间</CardTitle>
          <CardDescription>
            输入房间名称以加入现有房间
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="join-room-name">房间名称</Label>
            <Input
              id="join-room-name"
              value={roomName}
              onChange={(e) => {
                setRoomName(e.target.value);
                setError(null);
              }}
              placeholder="例如：project-alpha"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && roomName.trim()) {
                  handleJoinRoom();
                }
              }}
            />
          </div>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => {
                setMode("home");
                setRoomName("");
                setError(null);
              }}
              className="flex-1"
            >
              返回
            </Button>
            <Button
              onClick={handleJoinRoom}
              disabled={!roomName.trim()}
              className="flex-1"
            >
              加入房间
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
