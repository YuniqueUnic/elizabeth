"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, KeyRound, Loader2, RefreshCw, Slash } from "lucide-react";
import {
  createRoomIdentityCode,
  listRoomIdentityCodes,
  listRoomRoles,
  updateRoomIdentityCode,
  type RoomIdentityCode,
} from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { useToast } from "@/hooks/use-toast";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

const DURATION_UNIT_SECONDS = {
  second: 1,
  minute: 60,
  hour: 3600,
  day: 86400,
  month: 2592000,
} as const;

type DurationUnit = keyof typeof DURATION_UNIT_SECONDS;

function formatTime(value: string): string {
  const normalized = /[zZ]|[+-]\d{2}:?\d{2}$/.test(value) ? value : `${value}Z`;
  const date = new Date(normalized);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(undefined, { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function generateIdentityCode(): string {
  const values = crypto.getRandomValues(new Uint8Array(18));
  return `id-${Array.from(values, (value) => value.toString(36).padStart(2, "0")).join("")}`;
}

function roleLabel(role: string, translate: (key: string) => string): string {
  if (role === "admin") return translate("roleAdmin");
  if (role === "editor") return translate("roleEditor");
  if (role === "reader") return translate("roleReader");
  return role;
}

export function MemberAccessPanel({ roomName }: { roomName: string }) {
  const t = useTranslations("room.members");
  const tIdentity = useTranslations("room.identity");
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { token } = useRoomCapabilities();
  const [role, setRole] = useState("reader");
  const [durationValue, setDurationValue] = useState("7");
  const [durationUnit, setDurationUnit] = useState<DurationUnit>("day");
  const [revealedCode, setRevealedCode] = useState<string | null>(null);
  const [manualCopyValue, setManualCopyValue] = useState("");

  const codesQuery = useQuery({
    queryKey: ["room-identity-codes", roomName],
    queryFn: () => listRoomIdentityCodes(roomName, token ?? undefined),
    enabled: Boolean(token),
    staleTime: 15_000,
  });
  const rolesQuery = useQuery({
    queryKey: ["room-roles", roomName],
    queryFn: () => listRoomRoles(roomName, token ?? undefined),
    enabled: Boolean(token),
    staleTime: 15_000,
  });

  const invalidateCodes = () => queryClient.invalidateQueries({ queryKey: ["room-identity-codes", roomName] });
  const durationSeconds = (() => {
    const value = Number(durationValue);
    return Number.isFinite(value) && value > 0 ? Math.round(value * DURATION_UNIT_SECONDS[durationUnit]) : null;
  })();

  const create = useMutation({
    mutationFn: () => createRoomIdentityCode(roomName, {
      code: generateIdentityCode(),
      role,
      ...(role === "admin" ? {} : { expires_in_secs: durationSeconds ?? undefined }),
    }, token ?? undefined),
    onSuccess: (response) => {
      setRevealedCode(response.code ?? null);
      void invalidateCodes();
    },
    onError: () => toast({ title: t("issueFailed"), variant: "destructive" }),
  });
  const disable = useMutation({
    mutationFn: (id: number) => updateRoomIdentityCode(roomName, id, { disable: true }, token ?? undefined),
    onSuccess: () => {
      toast({ title: t("disableSuccess") });
      void invalidateCodes();
    },
    onError: () => toast({ title: t("disableFailed"), variant: "destructive" }),
  });
  const reset = useMutation({
    mutationFn: (id: number) => updateRoomIdentityCode(roomName, id, { code: generateIdentityCode() }, token ?? undefined),
    onSuccess: (response) => {
      setRevealedCode(response.code ?? null);
      void invalidateCodes();
    },
    onError: () => toast({ title: t("resetFailed"), variant: "destructive" }),
  });

  const copyCode = async (value: string) => {
    try {
      await copyTextToClipboard(value);
      toast({ title: tIdentity("copied") });
    } catch {
      setManualCopyValue(value);
    }
  };

  if (!token) return <p className="text-sm text-muted-foreground">{t("loading")}</p>;

  const roles = rolesQuery.data ?? [];
  const codes = codesQuery.data ?? [];
  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div>
          <h3 className="text-sm font-semibold">{t("createCode")}</h3>
          <p className="text-xs text-muted-foreground">{t("createCodeHint")}</p>
        </div>
        <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end">
          <div className="space-y-2">
            <Label htmlFor="identity-role">{t("roleLabel")}</Label>
            <Select value={role} onValueChange={setRole}>
              <SelectTrigger id="identity-role"><SelectValue /></SelectTrigger>
              <SelectContent>{roles.map((item) => <SelectItem key={item.role_key} value={item.role_key}>{item.display_name} ({item.role_key})</SelectItem>)}</SelectContent>
            </Select>
          </div>
          {role !== "admin" && <div className="grid grid-cols-[1fr_auto] gap-2">
            <div className="space-y-2"><Label htmlFor="identity-duration">{t("durationLabel")}</Label><Input id="identity-duration" type="number" min={1} value={durationValue} onChange={(event) => setDurationValue(event.target.value)} /></div>
            <Select value={durationUnit} onValueChange={(value) => setDurationUnit(value as DurationUnit)}><SelectTrigger className="self-end"><SelectValue /></SelectTrigger><SelectContent><SelectItem value="second">{t("unitSecond")}</SelectItem><SelectItem value="minute">{t("unitMinute")}</SelectItem><SelectItem value="hour">{t("unitHour")}</SelectItem><SelectItem value="day">{t("unitDay")}</SelectItem><SelectItem value="month">{t("unitMonth")}</SelectItem></SelectContent></Select>
          </div>}
          <Button type="button" onClick={() => {
            if (role !== "admin" && durationSeconds === null) { toast({ title: t("invalidDuration"), variant: "destructive" }); return; }
            create.mutate();
          }} disabled={create.isPending || !role || (role !== "admin" && durationSeconds === null)}>
            {create.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <KeyRound className="mr-2 h-4 w-4" />}{create.isPending ? t("issuing") : t("createCode")}
          </Button>
        </div>
        {role === "admin" && <p className="text-xs text-muted-foreground">{t("adminLifetimeHint")}</p>}
        {revealedCode && <div className="space-y-2 border border-primary/40 bg-primary/5 p-3">
          <Label>{t("issuedTokenLabel")}</Label>
          <div className="flex gap-2"><Input readOnly value={revealedCode} onFocus={(event) => event.currentTarget.select()} className="font-mono text-xs" /><Button type="button" size="icon" title={tIdentity("copy")} onClick={() => void copyCode(revealedCode)}><Copy className="h-4 w-4" /></Button></div>
          <p className="text-xs text-muted-foreground">{t("issuedTokenHint")}</p>
        </div>}
      </section>

      <section className="space-y-2 border-t pt-4">
        <div><h3 className="text-sm font-semibold">{t("identityCodes")}</h3><p className="text-xs text-muted-foreground">{t("identityCodesHint")}</p></div>
        {codesQuery.isLoading ? <p className="text-sm text-muted-foreground">{t("loading")}</p> : codes.length === 0 ? <p className="text-sm text-muted-foreground">{t("noIdentityCodes")}</p> : <div className="divide-y border px-3">
          {codes.map((item: RoomIdentityCode) => <div key={item.id} className="flex flex-col gap-3 py-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="min-w-0 space-y-1"><div className="flex flex-wrap items-center gap-2"><Badge variant={item.role === "admin" ? "default" : "secondary"}>{roleLabel(item.role, tIdentity)}</Badge>{item.revoked_at && <Badge variant="outline">{t("disabled")}</Badge>}<code className="text-xs text-muted-foreground">#{item.id}</code></div><p className="text-xs text-muted-foreground">{t("codeCreated", { time: formatTime(item.created_at) })} · {t("codeExpires", { time: formatTime(item.expires_at) })}</p></div>
            {!item.revoked_at && <div className="flex gap-2"><Button type="button" variant="outline" size="sm" onClick={() => reset.mutate(item.id)} disabled={reset.isPending}><RefreshCw className="mr-1.5 h-4 w-4" />{t("reset")}</Button><Button type="button" variant="outline" size="sm" className="text-destructive hover:text-destructive" onClick={() => disable.mutate(item.id)} disabled={disable.isPending}><Slash className="mr-1.5 h-4 w-4" />{t("disable")}</Button></div>}
          </div>)}
        </div>}
      </section>
      <ManualCopyDialog open={manualCopyValue.length > 0} value={manualCopyValue} onOpenChange={(open) => !open && setManualCopyValue("")} />
    </div>
  );
}
