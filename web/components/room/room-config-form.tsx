"use client";

import { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { RoomDetails } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { getPublicConfig } from "@/api/publicConfigService";
import { updateRoomSettings } from "@/api/roomService";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function RoomConfigForm({ roomDetails }: { roomDetails: RoomDetails }) {
  const t = useTranslations("room.config");
  const roomName = useAppStore((state) => state.currentRoomId);
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { can } = useRoomCapabilities();
  const [password, setPassword] = useState("");
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);
  const [removePassword, setRemovePassword] = useState(false);
  const config = useQuery({ queryKey: ["public-config"], queryFn: getPublicConfig, staleTime: Infinity });
  useEffect(() => setMaxViews(roomDetails.settings.maxViews), [roomDetails.settings.maxViews]);
  const mutation = useMutation({
    mutationFn: () => updateRoomSettings(roomName, { password: password || undefined, removePassword, maxViews }),
    onSuccess: (room) => { queryClient.setQueryData(["room", roomName], room); setPassword(""); setRemovePassword(false); toast({ title: t("save.successTitle") }); },
    onError: () => toast({ title: t("save.failTitle"), variant: "destructive" }),
  });
  const expiry = config.data?.room.expiry;
  return <div className="space-y-4">
    <h3 className="text-sm font-semibold">{t("title")}</h3>
    {!can.settings && <p className="text-xs text-muted-foreground">{t("adminOnly")}</p>}
    <div className="space-y-2"><Label htmlFor="room-password">{t("password.label")}</Label><Input id="room-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} disabled={!can.settings} autoComplete="new-password" placeholder={t("password.placeholder")} /></div>
    <div className="space-y-2"><Label htmlFor="room-max-views">{t("maxViews.label")}</Label><Input id="room-max-views" type="number" min={0} value={maxViews} onChange={(e) => setMaxViews(Number(e.target.value))} disabled={!can.settings} /></div>
    <Button type="button" variant="outline" onClick={() => setRemovePassword(true)} disabled={!can.settings || !roomDetails.settings.passwordProtected}>{t("password.removeAction")}</Button>
    <Button type="button" onClick={() => mutation.mutate()} disabled={!can.settings || mutation.isPending}>{mutation.isPending ? t("save.saving") : t("save.saveConfig")}</Button>
    {expiry && <p className="text-xs text-muted-foreground">{roomDetails.settings.expiresAt ?? t("expiry.placeholder")}</p>}
  </div>;
}
