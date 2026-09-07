"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { KeyRound, Loader2, UsersRound } from "lucide-react";
import { redeemRoomIdentityCode } from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RoomPermissionsDialog } from "@/components/room/room-permissions-dialog";
import { setRoomToken } from "@/lib/utils/api";

function RoleBadge({ roleKey, label }: { roleKey: string | null; label: string }) {
  return <Badge variant={roleKey === "admin" ? "default" : "secondary"}>{label}</Badge>;
}

export function IdentityCard({ roomName }: { roomName: string }) {
  const t = useTranslations("room.identity");
  const { can, roleKey } = useRoomCapabilities();
  const canManageMembers = can.manageRoles;
  const [permissionsOpen, setPermissionsOpen] = useState(false);
  const [redeemOpen, setRedeemOpen] = useState(false);
  const [redeemCode, setRedeemCode] = useState("");
  const [redeeming, setRedeeming] = useState(false);
  const [redeemError, setRedeemError] = useState<string | null>(null);

  const redeem = async () => {
    const code = redeemCode.trim();
    if (!code) return;

    setRedeemError(null);
    setRedeeming(true);
    try {
      const response = await redeemRoomIdentityCode(roomName, code);
      setRoomToken(roomName, {
        token: response.token,
        expiresAt: response.expires_at,
        capabilities: response.capabilities,
        roleKey: response.claims.role,
      });
      window.location.reload();
    } catch {
      setRedeemError(t("redeemInvalid"));
      setRedeeming(false);
    }
  };

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold">{t("title")}</h3>
        <RoleBadge
          roleKey={roleKey}
          label={
            roleKey === "admin"
              ? t("roleAdmin")
              : roleKey === "editor"
                ? t("roleEditor")
                : roleKey === "reader" || !roleKey
                  ? t("roleReader")
                  : t("roleCustom")
          }
        />
      </div>

      <p className="text-xs text-muted-foreground">
        {roleKey === "editor"
          ? t("editorHint")
          : roleKey === "reader" || !roleKey
            ? t("readerHint")
            : t("customHint")}
      </p>

      <div className="grid gap-2 sm:grid-cols-2">
        <Button type="button" variant="outline" className="justify-center gap-2" onClick={() => setRedeemOpen(true)}>
          <KeyRound className="h-4 w-4" />
          {t("redeemAction")}
        </Button>
        {canManageMembers && (
          <Button type="button" variant="outline" className="justify-center gap-2" onClick={() => setPermissionsOpen(true)}>
            <UsersRound className="h-4 w-4" />
            {t("managePermissions")}
          </Button>
        )}
      </div>

      <RoomPermissionsDialog roomName={roomName} open={permissionsOpen} onOpenChange={setPermissionsOpen} />
      <Dialog open={redeemOpen} onOpenChange={(open) => !redeeming && setRedeemOpen(open)}>
        <DialogContent className="sm:max-w-[420px]">
          <DialogHeader>
            <DialogTitle>{t("redeemTitle")}</DialogTitle>
            <DialogDescription>{t("redeemDescription")}</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="identity-redeem-code">{t("codeLabel")}</Label>
            <Input
              id="identity-redeem-code"
              type="password"
              autoComplete="off"
              value={redeemCode}
              onChange={(event) => {
                setRedeemCode(event.target.value);
                setRedeemError(null);
              }}
              onKeyDown={(event) => event.key === "Enter" && void redeem()}
              data-testid="identity-redeem-input"
            />
            {redeemError && <p role="alert" className="text-sm text-destructive">{redeemError}</p>}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRedeemOpen(false)} disabled={redeeming}>{t("redeemCancel")}</Button>
            <Button onClick={() => void redeem()} disabled={!redeemCode.trim() || redeeming}>
              {redeeming && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {redeeming ? t("redeeming") : t("redeemConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
