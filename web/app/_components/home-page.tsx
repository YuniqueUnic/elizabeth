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
import { ArrowRight, Eye, EyeOff, Lock, Plus } from "lucide-react";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { useTranslations } from "next-intl";

export default function HomePage() {
  const t = useTranslations("home");
  const tErrors = useTranslations("errors");
  const router = useRouter();
  const [mode, setMode] = useState<"home" | "create" | "join">("home");
  const [roomName, setRoomName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreateRoom = async () => {
    const trimmed = roomName.trim();
    if (!trimmed) {
      setError(tErrors("enterRoomName"));
      return;
    }

    if (trimmed.length < 3 || trimmed.length > 50) {
      setError(tErrors("roomNameLength3to50"));
      return;
    }

    const nameRegex = /^[a-zA-Z0-9](?:[a-zA-Z0-9_-]{1,48}[a-zA-Z0-9])?$/;
    if (!nameRegex.test(trimmed)) {
      setError(tErrors("roomNameFormat"));
      return;
    }

    // 验证密码：如果输入了密码，必须确认密码一致
    if (password && password !== confirmPassword) {
      setError(tErrors("passwordsDoNotMatch"));
      return;
    }

    // 如果只输入了确认密码而没有输入密码
    if (!password && confirmPassword) {
      setError(tErrors("enterPasswordFirst"));
      return;
    }

    try {
      setLoading(true);
      setError(null);

      // Create room with optional password
      await createRoom(trimmed, password || undefined);

      // Get access token
      await getAccessToken(trimmed, password || undefined);

      // Navigate to room
      router.push(`/${trimmed}`);
    } catch (err: any) {
      if (err.message?.includes("409") || err.message?.includes("exists")) {
        setError(tErrors("roomNameAlreadyExists"));
      } else {
        setError(err.message || tErrors("createRoomFailed"));
      }
    } finally {
      setLoading(false);
    }
  };

  const handleJoinRoom = () => {
    const trimmed = roomName.trim();
    if (!trimmed) {
      setError(tErrors("enterRoomName"));
      return;
    }

    if (trimmed.length < 3 || trimmed.length > 150) {
      setError(tErrors("roomNameLength3to150"));
      return;
    }

    const identifierRegex = /^[a-zA-Z0-9][a-zA-Z0-9_-]*[a-zA-Z0-9]$/;
    if (!identifierRegex.test(trimmed)) {
      setError(tErrors("roomNameFormat"));
      return;
    }

    // Navigate to room page, it will handle password if needed
    router.push(`/${trimmed}`);
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
              {t("platformDescription")}
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
                    <CardTitle>{t("createRoom")}</CardTitle>
                    <CardDescription>{t("createRoomDescription")}</CardDescription>
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
                    <CardTitle>{t("joinRoom")}</CardTitle>
                    <CardDescription>{t("joinRoomDescription")}</CardDescription>
                  </div>
                </div>
              </CardHeader>
            </Card>
          </div>

          <div className="text-center text-sm text-muted-foreground">
            <p>{t("noRegistrationNeeded")}</p>
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
            <CardTitle>{t("createRoomHeader")}</CardTitle>
            <CardDescription>
              {t("createRoomHeaderDescription")}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="room-name">{t("roomNameRequired")}</Label>
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
                placeholder={t("roomNamePlaceholder")}
                disabled={loading}
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">
                <div className="flex items-center gap-2">
                  <Lock className="h-4 w-4" />
                  <span>{t("passwordProtectionOptional")}</span>
                </div>
              </Label>
              <div className="relative">
                <Input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    setError(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && roomName.trim() && !loading) {
                      handleCreateRoom();
                    }
                  }}
                  placeholder={t("leaveEmptyNoPassword")}
                  disabled={loading}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="absolute right-0 top-0 h-full"
                  onClick={() => setShowPassword(!showPassword)}
                  disabled={loading}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirm-password">
                <div className="flex items-center gap-2">
                  <Lock className="h-4 w-4" />
                  <span>{t("confirmPassword")}</span>
                </div>
              </Label>
              <div className="relative">
                <Input
                  id="confirm-password"
                  type={showConfirmPassword ? "text" : "password"}
                  value={confirmPassword}
                  onChange={(e) => {
                    setConfirmPassword(e.target.value);
                    setError(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && roomName.trim() && !loading) {
                      handleCreateRoom();
                    }
                  }}
                  placeholder={password ? t("reenterPassword") : tErrors("enterPasswordFirst")}
                  disabled={loading || !password}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="absolute right-0 top-0 h-full"
                  onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                  disabled={loading || !password}
                >
                  {showConfirmPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
              </div>
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
                  setConfirmPassword("");
                  setShowPassword(false);
                  setShowConfirmPassword(false);
                  setError(null);
                }}
                disabled={loading}
                className="flex-1"
              >
                {t("back")}
              </Button>
              <Button
                onClick={handleCreateRoom}
                disabled={loading || !roomName.trim()}
                className="flex-1"
              >
                {loading ? t("creating") : t("createRoom")}
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
          <CardTitle>{t("joinRoomHeader")}</CardTitle>
          <CardDescription>
            {t("joinRoomHeaderDescription")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="join-room-name">{t("roomName")}</Label>
            <Input
              id="join-room-name"
              value={roomName}
              onChange={(e) => {
                setRoomName(e.target.value);
                setError(null);
              }}
              placeholder={t("roomNamePlaceholder")}
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
              {t("back")}
            </Button>
            <Button
              onClick={handleJoinRoom}
              disabled={!roomName.trim()}
              className="flex-1"
            >
              {t("joinRoom")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
