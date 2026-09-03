"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { getAccessToken, validateToken } from "@/api/authService";
import { getRoomToken, setRoomToken } from "@/lib/utils/api";
import { decodeJWT } from "@/lib/utils/jwt";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { RoomTokenClaims } from "@/types/generated/api.types";

export function TokenRoleDisplay({ roomName }: { roomName: string }) {
  const t = useTranslations("room.identityCode");
  const [identityCode, setIdentityCode] = useState("");
  const [claims, setClaims] = useState<RoomTokenClaims | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rotating, setRotating] = useState(false);
  const handleValidate = async () => {
    const value = identityCode.trim();
    setError(null);
    setClaims(null);
    if (!value || value.split(".").length !== 3 || !decodeJWT(value)) {
      setError(t("invalidFormat"));
      return;
    }
    try {
      const response = await validateToken(roomName, value);
      setClaims(response.claims);
    } catch {
      setError(t("invalid"));
    }
  };
  const rotateAdmin = async () => {
    setRotating(true);
    setError(null);
    try {
      const response = await getAccessToken(roomName);
      setRoomToken(roomName, { token: response.token, expiresAt: response.expires_at, refreshToken: response.refresh_token, capabilities: response.capabilities, roleKey: response.claims.role });
      setIdentityCode(response.token);
      setClaims(response.claims);
    } catch {
      setError(t("rotateFailed"));
    } finally {
      setRotating(false);
    }
  };

  return (
    <div className="space-y-2 rounded-lg border border-border/70 px-4 py-3">
      <Label htmlFor="room-identity-code">{t("label")}</Label>
      <div className="flex gap-2">
        <Input id="room-identity-code" value={identityCode} onChange={(event) => setIdentityCode(event.target.value)} type="password" autoComplete="off" placeholder={t("placeholder")} />
        <Button type="button" onClick={handleValidate} disabled={!identityCode.trim()}>{t("validate")}</Button>
      </div>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      {claims && <div className="flex items-center justify-between gap-2"><p className="text-sm text-muted-foreground">{t("role", { role: claims.role })}</p>{claims.role === "admin" && <Button type="button" variant="outline" size="sm" onClick={rotateAdmin} disabled={rotating}>{rotating ? t("rotating") : t("rotate")}</Button>}</div>}
    </div>
  );
}
