"use client";

import { useEffect, useMemo, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Info, Loader2, Plus } from "lucide-react";
import {
  createRoomRole,
  deleteRoomRole,
  listRoomRoles,
  updateRoomRole,
} from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { useToast } from "@/hooks/use-toast";
import type { Capability, Grant, RoleDefinition, Scope } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const NEW_ROLE = "__new__";

const CAPABILITY_GROUPS: { key: "room" | "msg" | "file"; capabilities: Capability[] }[] = [
  {
    key: "room",
    capabilities: ["room.share", "room.settings.update", "room.roles.manage", "room.delete"],
  },
  {
    key: "msg",
    capabilities: ["msg.read", "msg.send", "msg.copy", "msg.edit", "msg.delete"],
  },
  {
    key: "file",
    capabilities: [
      "file.list",
      "file.preview",
      "file.download",
      "file.upload",
      "file.delete",
      "file.policy.manage",
    ],
  },
];

const OWNABLE = new Set<Capability>(["msg.edit", "msg.delete", "file.delete"]);

interface RoleDraft {
  roleKey: string;
  displayName: string;
  grants: Grant[];
}

function draftFromRole(role: RoleDefinition): RoleDraft {
  return {
    roleKey: role.role_key,
    displayName: role.display_name,
    grants: role.capabilities.map((grant) => ({ ...grant })),
  };
}

function grantFor(grants: Grant[], capability: Capability): Grant | undefined {
  return grants.find((grant) => grant.capability === capability);
}

function CapabilityRow({
  capability,
  scope,
  onScopeChange,
}: {
  capability: Capability;
  scope: Scope | "off";
  onScopeChange: (scope: Scope | "off") => void;
}) {
  const t = useTranslations("room.roles");
  return (
    <div className="flex items-center justify-between gap-3 py-2">
      <div className="min-w-0">
        <p className="truncate text-sm">{t(`caps.${capability}.label`)}</p>
        <code className="text-[11px] text-muted-foreground">{capability}</code>
      </div>
      <Select
        value={scope}
        onValueChange={(value) => onScopeChange(value as Scope | "off")}
      >
        <SelectTrigger size="sm" className="w-24 shrink-0">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="off">{t("off")}</SelectItem>
          <SelectItem value="any">{t("any")}</SelectItem>
          {OWNABLE.has(capability) && <SelectItem value="own">{t("own")}</SelectItem>}
        </SelectContent>
      </Select>
    </div>
  );
}

export function RoleMatrixEditor({ roomName }: { roomName: string }) {
  const t = useTranslations("room.roles");
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { token, can } = useRoomCapabilities();

  const rolesQuery = useQuery({
    queryKey: ["room-roles", roomName],
    queryFn: () => listRoomRoles(roomName, token ?? undefined),
    enabled: Boolean(token),
    staleTime: 15_000,
  });
  const roles = useMemo(() => rolesQuery.data ?? [], [rolesQuery.data]);

  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [draft, setDraft] = useState<RoleDraft | null>(null);

  // 角色矩阵加载后默认选中 admin，避免右侧空白
  useEffect(() => {
    if (!selectedKey && roles.length > 0) {
      const initial = roles.find((role) => role.role_key === "admin") ?? roles[0];
      setSelectedKey(initial.role_key);
      setDraft(draftFromRole(initial));
    }
  }, [roles, selectedKey]);

  const selectRole = (role: RoleDefinition) => {
    setSelectedKey(role.role_key);
    setDraft(draftFromRole(role));
  };

  const startNewRole = () => {
    setSelectedKey(NEW_ROLE);
    setDraft({ roleKey: "", displayName: "", grants: [] });
  };

  const invalidateRoles = () =>
    queryClient.invalidateQueries({ queryKey: ["room-roles", roomName] });

  const save = useMutation({
    mutationFn: () => {
      if (!draft) throw new Error("empty draft");
      return selectedKey === NEW_ROLE
        ? createRoomRole(
            roomName,
            {
              role_key: draft.roleKey.trim(),
              display_name: draft.displayName.trim(),
              capabilities: draft.grants,
            },
            token ?? undefined,
          )
        : updateRoomRole(
            roomName,
            selectedKey!,
            { display_name: draft.displayName.trim(), capabilities: draft.grants },
            token ?? undefined,
          );
    },
    onSuccess: (saved) => {
      toast({ title: t("saveSuccess") });
      void invalidateRoles();
      setSelectedKey(saved.role_key);
      setDraft(draftFromRole(saved));
    },
    onError: (error: any) =>
      toast({
        title: t("saveFailed"),
        description: error?.message,
        variant: "destructive",
      }),
  });

  const remove = useMutation({
    mutationFn: (roleKey: string) => deleteRoomRole(roomName, roleKey, token ?? undefined),
    onSuccess: () => {
      toast({ title: t("deleteSuccess") });
      void invalidateRoles();
      setSelectedKey(null);
      setDraft(null);
    },
    onError: (error: any) => {
      const isDefaultRoleConflict = String(error?.message ?? "").includes("default join role");
      toast({
        title: isDefaultRoleConflict ? t("deleteDefaultRoleConflict") : t("deleteFailed"),
        variant: "destructive",
      });
    },
  });

  if (!token || rolesQuery.isLoading) {
    return <p className="py-4 text-sm text-muted-foreground">{t("loading")}</p>;
  }
  if (rolesQuery.isError || !can.manageRoles) {
    return <p className="py-4 text-sm text-destructive">{t("forbidden")}</p>;
  }

  const isNewRole = selectedKey === NEW_ROLE;
  const selectedRole = roles.find((role) => role.role_key === selectedKey);
  const canSave =
    Boolean(draft) &&
    draft!.displayName.trim().length > 0 &&
    (!isNewRole || draft!.roleKey.trim().length > 0);

  return (
    <div className="flex min-h-0 gap-4">
      {/* 角色列表 */}
      <div className="w-44 shrink-0 space-y-1">
        {roles.map((role) => (
          <button
            key={role.role_key}
            type="button"
            onClick={() => selectRole(role)}
            className={`flex w-full flex-col items-start gap-1 rounded-md px-3 py-2 text-left transition-colors ${
              selectedKey === role.role_key
                ? "bg-accent text-accent-foreground"
                : "hover:bg-accent/60"
            }`}
          >
            <span className="w-full truncate text-sm font-medium">{role.display_name}</span>
            <span className="flex flex-wrap items-center gap-1">
              {role.is_system && <Badge variant="secondary">{t("systemBadge")}</Badge>}
              <code className="text-[11px] text-muted-foreground">{role.role_key}</code>
            </span>
          </button>
        ))}
        <Button type="button" variant="outline" size="sm" className="w-full" onClick={startNewRole}>
          <Plus className="mr-1 h-4 w-4" />
          {t("newRole")}
        </Button>
      </div>

      {/* 编辑区 */}
      <div className="min-w-0 flex-1">
        {!draft ? (
          <div className="flex h-full items-center justify-center rounded-lg border border-dashed p-6 text-sm text-muted-foreground">
            {t("newRole")}
          </div>
        ) : (
          <div className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="role-display-name">{t("displayName")}</Label>
                <Input
                  id="role-display-name"
                  value={draft.displayName}
                  maxLength={64}
                  placeholder={t("displayNamePlaceholder")}
                  onChange={(event) => setDraft({ ...draft, displayName: event.target.value })}
                />
              </div>
              {isNewRole && (
                <div className="space-y-1.5">
                  <Label htmlFor="role-key">{t("roleKey")}</Label>
                  <Input
                    id="role-key"
                    value={draft.roleKey}
                    maxLength={64}
                    placeholder={t("roleKeyPlaceholder")}
                    onChange={(event) => setDraft({ ...draft, roleKey: event.target.value })}
                  />
                </div>
              )}
            </div>

            {selectedKey === "admin" && (
              <p className="flex items-start gap-1.5 text-xs text-amber-600 dark:text-amber-500">
                <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                {t("adminWarning")}
              </p>
            )}

            {CAPABILITY_GROUPS.map((group) => (
              <section key={group.key}>
                <h4 className="mb-1 text-xs font-medium text-muted-foreground">
                  {t(`groups.${group.key}`)}
                </h4>
                <div className="divide-y rounded-lg border px-3">
                  {group.capabilities.map((capability) => (
                    <CapabilityRow
                      key={capability}
                      capability={capability}
                      scope={grantFor(draft.grants, capability)?.scope ?? "off"}
                      onScopeChange={(scope) =>
                        setDraft((current) => {
                          if (!current) return current;
                          return {
                            ...current,
                            grants:
                              scope === "off"
                                ? current.grants.filter((grant) => grant.capability !== capability)
                                : [
                                    ...current.grants.filter(
                                      (grant) => grant.capability !== capability,
                                    ),
                                    { capability, scope } as Grant,
                                  ],
                          };
                        })
                      }
                    />
                  ))}
                </div>
              </section>
            ))}

            <div className="flex items-center justify-between gap-2 pt-1">
              <p className="text-xs text-muted-foreground">{t("immediateEffect")}</p>
              <div className="flex gap-2">
                {selectedRole && !selectedRole.is_system && (
                  <Button
                    type="button"
                    variant="outline"
                    className="text-destructive hover:text-destructive"
                    onClick={() => remove.mutate(selectedKey!)}
                    disabled={remove.isPending}
                  >
                    {remove.isPending ? t("deleting") : t("delete")}
                  </Button>
                )}
                <Button
                  type="button"
                  onClick={() => save.mutate()}
                  disabled={!canSave || save.isPending}
                >
                  {save.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      {t("saving")}
                    </>
                  ) : (
                    t("save")
                  )}
                </Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
