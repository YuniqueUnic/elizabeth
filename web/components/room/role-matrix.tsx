"use client";

import { useMemo, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createRoomRole, deleteRoomRole, listRoomRoles, updateRoomRole } from "@/api/roomService";
import { issueRoomRoleToken } from "@/api/authService";
import { getRoomToken } from "@/lib/utils/api";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import type { Capability, Grant, RoleDefinition, Scope } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

const capabilities: Capability[] = [
  "room.share", "room.settings.update", "room.roles.manage", "room.delete",
  "msg.read", "msg.send", "msg.copy", "msg.edit", "msg.delete",
  "file.list", "file.preview", "file.download", "file.upload", "file.delete", "file.policy.manage",
];
const ownable = new Set<Capability>(["msg.edit", "msg.delete", "file.delete"]);

const emptyRole = (roleKey = "") => ({ roleKey, displayName: "", grants: [] as Grant[] });

function grantFor(grants: Grant[], capability: Capability): Grant | undefined {
  return grants.find((grant) => grant.capability === capability);
}

export function RoleMatrix({ roomName }: { roomName: string }) {
  const t = useTranslations("room.roles");
  const queryClient = useQueryClient();
  const token = getRoomToken(roomName)?.token ?? undefined;
  const { can } = useRoomCapabilities();
  const rolesQuery = useQuery({
    queryKey: ["room-roles", roomName],
    queryFn: () => listRoomRoles(roomName, token),
    enabled: Boolean(token),
  });
  const [draft, setDraft] = useState(emptyRole());
  const [editing, setEditing] = useState<string | null>(null);
  const [issuedEditorToken, setIssuedEditorToken] = useState<string | null>(null);
  const roles = rolesQuery.data ?? [];
  const canManage = can.manageRoles;

  const save = useMutation({
    mutationFn: () => editing
      ? updateRoomRole(roomName, editing, { display_name: draft.displayName, capabilities: draft.grants }, token)
      : createRoomRole(roomName, { role_key: draft.roleKey, display_name: draft.displayName, capabilities: draft.grants }, token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["room-roles", roomName] });
      setDraft(emptyRole());
      setEditing(null);
    },
  });
  const issueEditor = useMutation({
    mutationFn: () => issueRoomRoleToken(roomName, "editor", token!),
    onSuccess: (response) => setIssuedEditorToken(response.token),
  });
  const remove = useMutation({
    mutationFn: (roleKey: string) => deleteRoomRole(roomName, roleKey, token),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["room-roles", roomName] }),
  });

  const selectRole = (role: RoleDefinition) => {
    setEditing(role.role_key);
    setDraft({ roleKey: role.role_key, displayName: role.display_name, grants: [...role.capabilities] });
  };
  const setGrant = (capability: Capability, scope: Scope | "off") => {
    setDraft((current) => ({
      ...current,
      grants: scope === "off"
        ? current.grants.filter((grant) => grant.capability !== capability)
        : [...current.grants.filter((grant) => grant.capability !== capability), { capability, scope }],
    }));
  };

  if (!token) return <p className="text-sm text-muted-foreground">{t("identityRequired")}</p>;
  if (rolesQuery.isLoading) return <p className="text-sm text-muted-foreground">{t("loading")}</p>;
  if (rolesQuery.isError) return <p className="text-sm text-destructive">{t("forbidden")}</p>;
  if (!canManage) return <p className="text-sm text-muted-foreground">{t("forbidden")}</p>;

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold">{t("title")}</h3>
      <div className="flex flex-wrap gap-2">
        {roles.map((role) => (
          <Button key={role.role_key} type="button" variant={editing === role.role_key ? "default" : "outline"} onClick={() => selectRole(role)}>
            {role.display_name}
          </Button>
        ))}
        <Button type="button" variant="ghost" onClick={() => { setEditing(null); setDraft(emptyRole()); }}>{t("newRole")}</Button>
      </div>
      <div className="space-y-3 rounded-lg border border-border/70 p-4">
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-1"><Label htmlFor="role-key">{t("roleKey")}</Label><Input id="role-key" placeholder={t("roleKeyPlaceholder")} value={draft.roleKey} disabled={Boolean(editing)} onChange={(event) => setDraft({ ...draft, roleKey: event.target.value })} /></div>
          <div className="space-y-1"><Label htmlFor="role-display-name">{t("displayName")}</Label><Input id="role-display-name" placeholder={t("displayNamePlaceholder")} value={draft.displayName} onChange={(event) => setDraft({ ...draft, displayName: event.target.value })} /></div>
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          {capabilities.map((capability) => {
            const grant = grantFor(draft.grants, capability);
            const scope = grant?.scope ?? "off";
            return <div key={capability} className="flex items-center justify-between gap-2 rounded border border-border/60 px-3 py-2"><code className="text-xs">{capability}</code><Select value={scope} onValueChange={(value) => setGrant(capability, value as Scope | "off")}><SelectTrigger className="w-28"><SelectValue /></SelectTrigger><SelectContent><SelectItem value="off">{t("off")}</SelectItem><SelectItem value="any">{t("any")}</SelectItem>{ownable.has(capability) && <SelectItem value="own">{t("own")}</SelectItem>}</SelectContent></Select></div>;
          })}
        </div>
        <div className="flex flex-wrap gap-2"><Button type="button" onClick={() => issueEditor.mutate()} disabled={issueEditor.isPending}>{issueEditor.isPending ? t("issuingEditor") : t("issueEditor")}</Button><Button type="button" onClick={() => save.mutate()} disabled={save.isPending || !draft.roleKey.trim() || !draft.displayName.trim()}>{save.isPending ? t("saving") : t("save")}</Button>{editing && !roles.find((role) => role.role_key === editing)?.is_system && <Button type="button" variant="destructive" onClick={() => remove.mutate(editing)} disabled={remove.isPending}>{remove.isPending ? t("deleting") : t("delete")}</Button>}</div>
      </div>
      {issuedEditorToken && <div className="space-y-2 rounded border border-border/60 p-3"><Label htmlFor="issued-editor-token">{t("issuedToken")}</Label><Input id="issued-editor-token" value={issuedEditorToken} readOnly type="password" onFocus={(event) => event.currentTarget.select()} /><p className="text-xs text-muted-foreground">{t("issuedTokenHint")}</p></div>}
    </div>
  );
}
