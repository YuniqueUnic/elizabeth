"use client";

import { useMemo, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, KeyRound, Loader2, Pencil, RefreshCw, Slash, WandSparkles } from "lucide-react";
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
import { clearRoomToken } from "@/lib/utils/api";
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
type RevealedCode = { code: string; role: string; sessionEnded: boolean };

function formatTime(value: string): string {
  const normalized = /[zZ]|[+-]\d{2}:?\d{2}$/.test(value) ? value : `${value}Z`;
  const date = new Date(normalized);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString(undefined, { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
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

function durationInSeconds(value: string, unit: DurationUnit): number | null {
  const amount = Number(value);
  return Number.isFinite(amount) && amount > 0
    ? Math.round(amount * DURATION_UNIT_SECONDS[unit])
    : null;
}

function DurationFields({
  value,
  unit,
  onValueChange,
  onUnitChange,
  testIdPrefix,
  t,
}: {
  value: string;
  unit: DurationUnit;
  onValueChange: (value: string) => void;
  onUnitChange: (value: DurationUnit) => void;
  testIdPrefix?: string;
  t: ReturnType<typeof useTranslations>;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
      <div className="space-y-2">
        <Label htmlFor={`${testIdPrefix ?? "identity"}-duration`}>{t("durationLabel")}</Label>
        <Input
          id={`${testIdPrefix ?? "identity"}-duration`}
          data-testid={testIdPrefix ? `${testIdPrefix}-duration-value` : undefined}
          type="number"
          min={1}
          value={value}
          onChange={(event) => onValueChange(event.target.value)}
        />
      </div>
      <Select value={unit} onValueChange={(next) => onUnitChange(next as DurationUnit)}>
        <SelectTrigger data-testid={testIdPrefix ? `${testIdPrefix}-duration-unit` : undefined} className="self-end">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="second">{t("unitSecond")}</SelectItem>
          <SelectItem value="minute">{t("unitMinute")}</SelectItem>
          <SelectItem value="hour">{t("unitHour")}</SelectItem>
          <SelectItem value="day">{t("unitDay")}</SelectItem>
          <SelectItem value="month">{t("unitMonth")}</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}

export function MemberAccessPanel({ roomName }: { roomName: string }) {
  const t = useTranslations("room.members");
  const tIdentity = useTranslations("room.identity");
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { token } = useRoomCapabilities();
  const [role, setRole] = useState("reader");
  const [newCode, setNewCode] = useState("");
  const [newDurationValue, setNewDurationValue] = useState("7");
  const [newDurationUnit, setNewDurationUnit] = useState<DurationUnit>("day");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [replacementCode, setReplacementCode] = useState("");
  const [editDurationValue, setEditDurationValue] = useState("7");
  const [editDurationUnit, setEditDurationUnit] = useState<DurationUnit>("day");
  const [revealedCode, setRevealedCode] = useState<RevealedCode | null>(null);
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
  const newDurationSeconds = useMemo(
    () => durationInSeconds(newDurationValue, newDurationUnit),
    [newDurationUnit, newDurationValue],
  );
  const editDurationSeconds = useMemo(
    () => durationInSeconds(editDurationValue, editDurationUnit),
    [editDurationUnit, editDurationValue],
  );

  const reveal = (code: string | undefined, item: RoomIdentityCode) => {
    if (!code) return;
    const sessionEnded = item.is_current;
    setRevealedCode({ code, role: item.role, sessionEnded });
    if (sessionEnded) {
      clearRoomToken(roomName);
    }
  };

  const create = useMutation({
    mutationFn: () => createRoomIdentityCode(roomName, {
      code: newCode.trim(),
      role,
      ...(role === "admin" ? {} : { expires_in_secs: newDurationSeconds ?? undefined }),
    }, token ?? undefined),
    onSuccess: (response) => {
      reveal(response.code, response.identity_code);
      setNewCode("");
      void invalidateCodes();
    },
    onError: () => toast({ title: t("issueFailed"), variant: "destructive" }),
  });
  const disable = useMutation({
    mutationFn: (item: RoomIdentityCode) => updateRoomIdentityCode(roomName, item.id, { disable: true }, token ?? undefined),
    onSuccess: (response) => {
      if (response.identity_code.is_current) clearRoomToken(roomName);
      toast({ title: t("disableSuccess") });
      void invalidateCodes();
    },
    onError: () => toast({ title: t("disableFailed"), variant: "destructive" }),
  });
  const update = useMutation({
    mutationFn: (item: RoomIdentityCode) => updateRoomIdentityCode(roomName, item.id, {
      ...(replacementCode.trim() ? { code: replacementCode.trim() } : {}),
      ...(item.role === "admin" ? {} : { expires_in_secs: editDurationSeconds ?? undefined }),
    }, token ?? undefined),
    onSuccess: (response) => {
      reveal(response.code, response.identity_code);
      setEditingId(null);
      setReplacementCode("");
      void invalidateCodes();
    },
    onError: () => toast({ title: t("updateFailed"), variant: "destructive" }),
  });

  const copyCode = async (value: string) => {
    try {
      await copyTextToClipboard(value);
      toast({ title: tIdentity("copied") });
    } catch {
      setManualCopyValue(value);
    }
  };

  const beginEdit = (item: RoomIdentityCode) => {
    setEditingId(item.id);
    setReplacementCode("");
    setEditDurationValue("7");
    setEditDurationUnit("day");
  };

  if (!token && !revealedCode) return <p className="text-sm text-muted-foreground">{t("loading")}</p>;

  const roles = rolesQuery.data ?? [];
  const codes = codesQuery.data ?? [];
  const newCodeValid = newCode.trim().length >= 6;
  return (
    <div className="space-y-6">
      {revealedCode?.sessionEnded && (
        <section className="space-y-3 border border-amber-500/40 bg-amber-500/5 p-3" role="status">
          <p className="text-sm font-medium">{t("currentCodeSessionEnded")}</p>
          <p className="text-xs text-muted-foreground">{t("currentCodeSessionEndedHint")}</p>
        </section>
      )}

      <section className="space-y-3">
        <div>
          <h3 className="text-sm font-semibold">{t("createCode")}</h3>
          <p className="text-xs text-muted-foreground">{t("createCodeHint")}</p>
        </div>
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="identity-role">{t("roleLabel")}</Label>
            <Select value={role} onValueChange={setRole} disabled={Boolean(revealedCode?.sessionEnded)}>
              <SelectTrigger id="identity-role"><SelectValue /></SelectTrigger>
              <SelectContent>{roles.map((item) => <SelectItem key={item.role_key} value={item.role_key}>{item.display_name} ({item.role_key})</SelectItem>)}</SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="new-identity-code">{t("codeInputLabel")}</Label>
            <div className="flex gap-2">
              <Input id="new-identity-code" value={newCode} autoComplete="off" onChange={(event) => setNewCode(event.target.value)} disabled={Boolean(revealedCode?.sessionEnded)} />
              <Button type="button" size="icon" variant="outline" title={t("generateCode")} onClick={() => setNewCode(generateIdentityCode())} disabled={Boolean(revealedCode?.sessionEnded)}>
                <WandSparkles className="h-4 w-4" />
              </Button>
            </div>
          </div>
          {role !== "admin" && <DurationFields value={newDurationValue} unit={newDurationUnit} onValueChange={setNewDurationValue} onUnitChange={setNewDurationUnit} testIdPrefix="editor-token" t={t} />}
        </div>
        {role === "admin" && <p className="text-xs text-muted-foreground">{t("adminLifetimeHint")}</p>}
        <Button type="button" onClick={() => create.mutate()} disabled={Boolean(revealedCode?.sessionEnded) || create.isPending || !role || !newCodeValid || (role !== "admin" && newDurationSeconds === null)}>
          {create.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <KeyRound className="mr-2 h-4 w-4" />}
          {create.isPending ? t("issuing") : t("createCode")}
        </Button>
      </section>

      {revealedCode && (
        <section className="space-y-2 border border-primary/40 bg-primary/5 p-3">
          <Label>{t("issuedCodeLabel", { role: roleLabel(revealedCode.role, tIdentity) })}</Label>
          <div className="flex gap-2">
            <Input readOnly value={revealedCode.code} onFocus={(event) => event.currentTarget.select()} className="font-mono text-xs" />
            <Button type="button" size="icon" title={tIdentity("copy")} onClick={() => void copyCode(revealedCode.code)}><Copy className="h-4 w-4" /></Button>
          </div>
          <p className="text-xs text-muted-foreground">{t("issuedCodeHint")}</p>
          {revealedCode.sessionEnded && <Button type="button" onClick={() => window.location.reload()}>{t("returnToRedeem")}</Button>}
        </section>
      )}

      <section className="space-y-2 border-t pt-4">
        <div><h3 className="text-sm font-semibold">{t("identityCodes")}</h3><p className="text-xs text-muted-foreground">{t("identityCodesHint")}</p></div>
        {codesQuery.isLoading ? <p className="text-sm text-muted-foreground">{t("loading")}</p> : codes.length === 0 ? <p className="text-sm text-muted-foreground">{t("noIdentityCodes")}</p> : <div className="divide-y border px-3">
          {codes.map((item) => <div key={item.id} data-testid={item.is_current ? "current-identity-code" : `identity-code-${item.id}`} className="space-y-3 py-3">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="min-w-0 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={item.role === "admin" ? "default" : "secondary"}>{roleLabel(item.role, tIdentity)}</Badge>
                  {item.is_current && <Badge variant="outline">{t("currentCode")}</Badge>}
                  {item.revoked_at && <Badge variant="outline">{t("disabled")}</Badge>}
                  <code className="text-xs text-muted-foreground">#{item.id}</code>
                </div>
                <p className="text-xs text-muted-foreground">{t("codeCreated", { time: formatTime(item.created_at) })} · {t("codeExpires", { time: formatTime(item.expires_at) })}</p>
              </div>
              {!item.revoked_at && !revealedCode?.sessionEnded && <div className="flex gap-2">
                <Button type="button" variant="outline" size="sm" onClick={() => beginEdit(item)} disabled={update.isPending}><Pencil className="mr-1.5 h-4 w-4" />{t("replace")}</Button>
                <Button type="button" variant="outline" size="sm" className="text-destructive hover:text-destructive" onClick={() => disable.mutate(item)} disabled={disable.isPending}><Slash className="mr-1.5 h-4 w-4" />{t("disable")}</Button>
              </div>}
            </div>
            {editingId === item.id && !item.revoked_at && <div className="grid gap-3 border-t pt-3 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor={`replace-identity-code-${item.id}`}>{t("replacementCodeLabel")}</Label>
                <div className="flex gap-2">
                  <Input id={`replace-identity-code-${item.id}`} value={replacementCode} autoComplete="off" onChange={(event) => setReplacementCode(event.target.value)} />
                  <Button type="button" size="icon" variant="outline" title={t("generateCode")} onClick={() => setReplacementCode(generateIdentityCode())}><RefreshCw className="h-4 w-4" /></Button>
                </div>
              </div>
              {item.role !== "admin" && <DurationFields value={editDurationValue} unit={editDurationUnit} onValueChange={setEditDurationValue} onUnitChange={setEditDurationUnit} t={t} />}
              {item.role === "admin" && <p className="self-end text-xs text-muted-foreground">{t("adminLifetimeHint")}</p>}
              <div className="flex gap-2 sm:col-span-2">
                <Button type="button" onClick={() => update.mutate(item)} disabled={update.isPending || (!replacementCode.trim() && item.role === "admin") || (item.role !== "admin" && editDurationSeconds === null)}>
                  {update.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}{t("saveCode")}
                </Button>
                <Button type="button" variant="outline" onClick={() => setEditingId(null)} disabled={update.isPending}>{t("cancel")}</Button>
              </div>
            </div>}
          </div>)}
        </div>}
      </section>
      <ManualCopyDialog open={manualCopyValue.length > 0} value={manualCopyValue} onOpenChange={(open) => !open && setManualCopyValue("")} />
    </div>
  );
}
