"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { Copy, KeyRound, Loader2, RotateCw, UsersRound } from "lucide-react";
import { getAccessToken, validateToken } from "@/api/authService";
import { getRoomTokenString } from "@/lib/utils/api";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { useToast } from "@/hooks/use-toast";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import { RoomPermissionsDialog } from "@/components/room/room-permissions-dialog";
import { Badge } from "@/components/ui/badge";
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
import { setRoomToken } from "@/lib/utils/api";
import { Button } from "@/components/ui/button";

function RoleBadge({ roleKey, label }: { roleKey: string | null; label: string }) {
  return (
    <Badge variant={roleKey === "admin" ? "default" : "secondary"} className="gap-1">
      {label}
    </Badge>
  );
}

/**
 * 侧边栏顶部身份卡：显示当前角色；管理员可复制/轮换自己的身份码，
 * 并从这里进入「成员与权限」对话框完成分发与角色配置。
 */
export function IdentityCard({ roomName }: { roomName: string }) {
  const t = useTranslations("room.identity");
  const { toast } = useToast();
  const { can, roleKey } = useRoomCapabilities();
  const isAdmin = roleKey === "admin";

  const [rotating, setRotating] = useState(false);
  const [permissionsOpen, setPermissionsOpen] = useState(false);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const [redeemOpen, setRedeemOpen] = useState(false);
  const [redeemCode, setRedeemCode] = useState("");
  const [redeeming, setRedeeming] = useState(false);
  const [redeemError, setRedeemError] = useState<string | null>(null);
  const token = getRoomTokenString(roomName);

  // 身份码兑换：粘贴 admin 分发的身份码，POST /tokens 换取正式会话并切换角色
  const redeem = async () => {
    const code = redeemCode.trim();
    setRedeemError(null);
    if (!code) return;
    setRedeeming(true);
    try {
      const response = await validateToken(roomName, code);
      setRoomToken(roomName, {
        token: code,
        expiresAt: new Date(Number(response.claims.exp) * 1000).toISOString(),
        roleKey: response.claims.role,
      });
      window.location.reload();
    } catch {
      setRedeemError(t("redeemInvalid"));
      setRedeeming(false);
    }
  };

  const rotate = async () => {
    setRotating(true);
    try {
      // getAccessToken 会携带当前 token 请求续签，后端原子轮换 jti 并更新本地存储
      await getAccessToken(roomName);
      toast({
        title: t("rotateSuccessTitle"),
        description: t("rotateSuccessDescription"),
      });
      // 旧 jti 已失效，页面内 WebSocket 仍持有旧 token；整页刷新以新身份码重连
      window.setTimeout(() => window.location.reload(), 900);
    } catch {
      setRotating(false);
      toast({ title: t("rotateFailed"), variant: "destructive" });
    }
  };

  const copy = async () => {
    if (!token) return;
    try {
      await copyTextToClipboard(token);
      toast({ title: t("copied") });
    } catch {
      setManualCopyValue(token);
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

      {isAdmin ? (
        <>
          <div className="flex gap-2">
            <Input
              readOnly
              type="password"
              value={token ?? ""}
              onFocus={(event) => event.currentTarget.select()}
              aria-label={t("codeLabel")}
              className="font-mono text-xs"
            />
            <Button
              type="button"
              variant="outline"
              size="icon"
              title={t("copy")}
              onClick={() => void copy()}
            >
              <Copy className="h-4 w-4" />
            </Button>
            <Button
              type="button"
              variant="outline"
              size="icon"
              title={t("rotate")}
              onClick={() => void rotate()}
              disabled={rotating}
            >
              {rotating ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RotateCw className="h-4 w-4" />
              )}
            </Button>
          </div>
          <Button
            type="button"
            variant="outline"
            className="w-full justify-center gap-2"
            onClick={() => setPermissionsOpen(true)}
          >
            <UsersRound className="h-4 w-4" />
            {t("managePermissions")}
          </Button>
        </>
      ) : (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground">
            {roleKey === "editor"
              ? t("editorHint")
              : roleKey === "reader" || !roleKey
                ? t("readerHint")
                : t("customHint")}
          </p>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="w-full justify-center gap-2"
            onClick={() => setRedeemOpen(true)}
          >
            <KeyRound className="h-4 w-4" />
            {t("redeemAction")}
          </Button>
        </div>
      )}

      <RoomPermissionsDialog
        roomName={roomName}
        open={permissionsOpen}
        onOpenChange={setPermissionsOpen}
      />
      <Dialog open={redeemOpen} onOpenChange={(open) => {
        if (!redeeming) setRedeemOpen(open);
      }}>
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
              onKeyDown={(event) => {
                if (event.key === "Enter") void redeem();
              }}
              data-testid="identity-redeem-input"
            />
            {redeemError && (
              <p role="alert" className="text-sm text-destructive">{redeemError}</p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRedeemOpen(false)} disabled={redeeming}>
              {t("redeemCancel")}
            </Button>
            <Button onClick={() => void redeem()} disabled={!redeemCode.trim() || redeeming}>
              {redeeming ? t("redeeming") : t("redeemConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setManualCopyValue("");
        }}
      />
    </section>
  );
}
