"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Loader2, ShieldCheck } from "lucide-react";
import { listRoomTokens } from "@/api/roomService";
import { issueRoomRoleToken, revokeRoomToken, validateToken } from "@/api/authService";
import { getRoomTokenString } from "@/lib/utils/api";
import { decodeJWT } from "@/lib/utils/jwt";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { useToast } from "@/hooks/use-toast";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import { MAX_EDITOR_TOKENS, type RoomTokenView } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/** 后端 naive datetime 为 UTC；前端解析时补 Z 并截断到毫秒。 */
function parseNaiveUtc(value: string): Date | null {
  const normalized = value.replace(/(\.\d{3})\d+$/, "$1");
  const date = new Date(value.includes("T") ? `${normalized}Z` : normalized);
  return Number.isNaN(date.getTime()) ? null : date;
}

function formatTime(value: string): string {
  const date = parseNaiveUtc(value);
  if (!date) return value;
  return date.toLocaleString(undefined, {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function isActiveSession(token: RoomTokenView): boolean {
  if (token.revoked_at) return false;
  const expiresAt = parseNaiveUtc(token.expires_at);
  return expiresAt ? expiresAt.getTime() > Date.now() : false;
}

function roleLabel(roleKey: string | null | undefined, translate: (key: string) => string): string {
  // translate 已限定在 room.identity 命名空间内
  switch (roleKey) {
    case "admin":
      return translate("roleAdmin");
    case "editor":
      return translate("roleEditor");
    case "reader":
      return translate("roleReader");
    default:
      return roleKey || "-";
  }
}

interface SessionRowProps {
  token: RoomTokenView;
  isCurrent: boolean;
  onRevoke: () => void;
  revoking: boolean;
}

function SessionRow({ token, isCurrent, onRevoke, revoking }: SessionRowProps) {
  const t = useTranslations("room.members");
  const tIdentity = useTranslations("room.identity");

  return (
    <div className="flex items-center justify-between gap-3 py-2.5">
      <div className="flex min-w-0 flex-col gap-1">
        <div className="flex items-center gap-2">
          <Badge variant={token.role_key === "admin" ? "default" : "secondary"}>
            {roleLabel(token.role_key, tIdentity)}
          </Badge>
          {isCurrent && <Badge variant="outline">{t("sessionCurrent")}</Badge>}
          <code className="truncate text-xs text-muted-foreground">
            {token.jti.slice(0, 8)}…
          </code>
        </div>
        <p className="text-xs text-muted-foreground">
          {t("sessionCreated", { time: formatTime(token.created_at) })}
          {" · "}
          {t("sessionExpires", { time: formatTime(token.expires_at) })}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="shrink-0 text-destructive hover:text-destructive"
        disabled={isCurrent || revoking}
        title={isCurrent ? t("sessionCurrent") : t("revoke")}
        onClick={onRevoke}
      >
        {revoking ? t("revoking") : t("revoke")}
      </Button>
    </div>
  );
}

export function MemberAccessPanel({ roomName }: { roomName: string }) {
  const t = useTranslations("room.members");
  const tIdentity = useTranslations("room.identity");
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { token, payload } = useRoomCapabilities();
  const currentJti = payload?.jti ?? null;

  const tokensQuery = useQuery({
    queryKey: ["room-tokens", roomName],
    queryFn: () => listRoomTokens(roomName, token ?? undefined),
    enabled: Boolean(token),
    staleTime: 15_000,
  });

  const [issuedCode, setIssuedCode] = useState<string | null>(null);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const [codeToVerify, setCodeToVerify] = useState("");
  const [verifying, setVerifying] = useState(false);
  const [verifyResult, setVerifyResult] = useState<
    { ok: true; role: string } | { ok: false; reason: "format" | "invalid" } | null
  >(null);

  const sessions = (tokensQuery.data ?? []).filter(isActiveSession);
  const activeEditorSeats = sessions.filter((item) => item.role_key === "editor").length;

  const invalidateTokens = () =>
    queryClient.invalidateQueries({ queryKey: ["room-tokens", roomName] });

  const issue = useMutation({
    mutationFn: () => issueRoomRoleToken(roomName, "editor", token!),
    onSuccess: (response) => {
      setIssuedCode(response.token);
      void invalidateTokens();
    },
    onError: (error: any) => {
      const seatsFull = String(error?.message ?? "").includes("Maximum");
      toast({
        title: seatsFull
          ? t("seatsFull", { max: MAX_EDITOR_TOKENS })
          : t("issueFailed"),
        variant: "destructive",
      });
    },
  });

  const revoke = useMutation({
    mutationFn: (jti: string) => revokeRoomToken(roomName, jti, token ?? undefined),
    onSuccess: () => {
      toast({ title: t("revokeSuccess") });
      void invalidateTokens();
    },
    onError: () => toast({ title: t("revokeFailed"), variant: "destructive" }),
  });

  const copyCode = async (value: string) => {
    try {
      await copyTextToClipboard(value);
      toast({ title: tIdentity("copied") });
    } catch {
      setManualCopyValue(value);
    }
  };

  const handleVerify = async () => {
    const value = codeToVerify.trim();
    setVerifyResult(null);
    if (!value || value.split(".").length !== 3 || !decodeJWT(value)) {
      setVerifyResult({ ok: false, reason: "format" });
      return;
    }
    setVerifying(true);
    try {
      const response = await validateToken(roomName, value);
      setVerifyResult({ ok: true, role: response.claims.role });
    } catch {
      setVerifyResult({ ok: false, reason: "invalid" });
    } finally {
      setVerifying(false);
    }
  };

  if (!token) {
    return <p className="text-sm text-muted-foreground">{t("loading")}</p>;
  }

  return (
    <div className="space-y-6">
      {/* Editor 席位 */}
      <section className="space-y-3">
        <div className="flex items-baseline justify-between gap-2">
          <h3 className="text-sm font-semibold">{t("editorSeats")}</h3>
          <span
            className={`text-sm tabular-nums ${
              activeEditorSeats >= MAX_EDITOR_TOKENS ? "font-medium text-destructive" : "text-muted-foreground"
            }`}
          >
            {t("editorSeatsUsage", { used: activeEditorSeats, max: MAX_EDITOR_TOKENS })}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">{t("editorSeatsHint")}</p>
        <Button
          type="button"
          size="sm"
          onClick={() => issue.mutate()}
          disabled={issue.isPending || activeEditorSeats >= MAX_EDITOR_TOKENS}
        >
          {issue.isPending ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              {t("issuing")}
            </>
          ) : (
            <>
              <KeyRound className="mr-2 h-4 w-4" />
              {t("issueEditor")}
            </>
          )}
        </Button>

        {issuedCode && (
          <div className="space-y-2 rounded-lg border border-primary/40 bg-primary/5 p-3">
            <Label className="text-sm font-medium">{t("issuedTokenLabel")}</Label>
            <div className="flex gap-2">
              <Input
                readOnly
                type="password"
                value={issuedCode}
                onFocus={(event) => event.currentTarget.select()}
                className="font-mono text-xs"
              />
              <Button type="button" size="sm" onClick={() => copyCode(issuedCode)}>
                {tIdentity("copy")}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{t("issuedTokenHint")}</p>
          </div>
        )}
      </section>

      {/* 活跃会话 */}
      <section className="space-y-2">
        <h3 className="text-sm font-semibold">{t("sessions")}</h3>
        <p className="text-xs text-muted-foreground">{t("sessionsHint")}</p>
        {tokensQuery.isLoading ? (
          <p className="py-2 text-sm text-muted-foreground">{t("loading")}</p>
        ) : sessions.length === 0 ? (
          <p className="py-2 text-sm text-muted-foreground">{t("noSessions")}</p>
        ) : (
          <div className="divide-y rounded-lg border px-3">
            {sessions.map((item) => (
              <SessionRow
                key={item.jti}
                token={item}
                isCurrent={item.jti === currentJti}
                onRevoke={() => revoke.mutate(item.jti)}
                revoking={revoke.isPending && revoke.variables === item.jti}
              />
            ))}
          </div>
        )}
      </section>

      {/* 身份码校验 */}
      <section className="space-y-2 border-t pt-4">
        <h3 className="flex items-center gap-1.5 text-sm font-semibold">
          <ShieldCheck className="h-4 w-4" />
          {t("validator")}
        </h3>
        <p className="text-xs text-muted-foreground">{t("validatorHint")}</p>
        <div className="flex gap-2">
          <Input
            type="password"
            autoComplete="off"
            value={codeToVerify}
            onChange={(event) => {
              setCodeToVerify(event.target.value);
              setVerifyResult(null);
            }}
            placeholder={t("validatorPlaceholder")}
            onKeyDown={(event) => {
              if (event.key === "Enter") void handleVerify();
            }}
            className="font-mono text-xs"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="shrink-0"
            onClick={() => void handleVerify()}
            disabled={!codeToVerify.trim() || verifying}
          >
            {verifying ? t("validating") : t("validate")}
          </Button>
        </div>
        {verifyResult?.ok && (
          <p className="text-sm font-medium text-foreground">
            {t("validRole", { role: roleLabel(verifyResult.role, tIdentity) })}
          </p>
        )}
        {verifyResult && !verifyResult.ok && (
          <p role="alert" className="text-sm text-destructive">
            {verifyResult.reason === "format" ? t("invalidFormat") : t("invalid")}
          </p>
        )}
      </section>

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
