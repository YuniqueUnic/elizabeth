"use client";

import { useState } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Eye, EyeOff } from "lucide-react";
import type { RoomDetails } from "@/lib/types";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { updateRoomPermissions, updateRoomSettings } from "@/api/roomService";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { useRoomPermissions } from "@/hooks/use-room-permissions";

interface RoomSettingsFormProps {
  roomDetails: RoomDetails;
}

const EXPIRY_OPTIONS = [
  { label: "1 分钟", value: "1min", ms: 60 * 1000 },
  { label: "10 分钟", value: "10min", ms: 10 * 60 * 1000 },
  { label: "1 小时", value: "1hr", ms: 60 * 60 * 1000 },
  { label: "12 小时", value: "12hr", ms: 12 * 60 * 60 * 1000 },
  { label: "1 天", value: "1day", ms: 24 * 60 * 60 * 1000 },
  { label: "1 周", value: "1week", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "永不过期", value: "never", ms: 0 },
];

export function RoomSettingsForm({ roomDetails }: RoomSettingsFormProps) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { can } = useRoomPermissions();

  const [expiryOption, setExpiryOption] = useState("1day");
  const [password, setPassword] = useState(roomDetails.settings.password || "");
  const [showPassword, setShowPassword] = useState(false);
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);

  // 只有拥有删除权限的用户才能修改房间设置
  const canModifySettings = can.delete;

  const updateMutation = useMutation({
    mutationFn: (settings: {
      password?: string | null;
      expiresAt?: string | null;
      maxViews?: number;
    }) => updateRoomSettings(currentRoomId, settings),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] });
      toast({
        title: "设置已保存",
        description: "房间设置已成功更新",
      });
    },
    onError: () => {
      toast({
        title: "保存失败",
        description: "无法保存房间设置，请重试",
        variant: "destructive",
      });
    },
  });

  const handleSave = () => {
    const option = EXPIRY_OPTIONS.find((opt) => opt.value === expiryOption);
    const expiresAt = option?.ms === 0
      ? null
      : new Date(Date.now() + (option?.ms || 0)).toISOString().slice(0, 23);

    updateMutation.mutate({
      expiresAt: expiresAt ?? undefined,
      password: password.length > 0 ? password : null,
      maxViews,
    });
  };

  return (
    <div className="space-y-4">
      <div>
        <h3 className="mb-3 text-sm font-semibold">房间设置</h3>

        {!canModifySettings && (
          <p className="text-xs text-muted-foreground mb-3 p-2 bg-muted rounded-md">
            只有房间管理员（拥有删除权限）可以修改房间设置
          </p>
        )}

        <div className="space-y-2 mt-2">
          <Label htmlFor="expires-at">过期时间</Label>
          <Select
            value={expiryOption}
            onValueChange={setExpiryOption}
            disabled={!canModifySettings}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="选择过期时间" />
            </SelectTrigger>
            <SelectContent>
              {EXPIRY_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Password */}
        <div className="space-y-2 mt-2">
          <Label htmlFor="password">房间密码</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="设置房间密码"
              disabled={!canModifySettings}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-0 top-0 h-full"
              onClick={() => setShowPassword(!showPassword)}
              disabled={!canModifySettings}
            >
              {showPassword
                ? <EyeOff className="h-4 w-4" />
                : <Eye className="h-4 w-4" />}
            </Button>
          </div>
        </div>

        {/* Max Views */}
        <div className="space-y-2 mt-2">
          <Label htmlFor="max-views">最大查看次数</Label>
          <Input
            id="max-views"
            type="number"
            value={maxViews}
            onChange={(e) => setMaxViews(Number(e.target.value))}
            min={1}
            disabled={!canModifySettings}
          />
        </div>

        <Button
          onClick={handleSave}
          className="mt-4 w-full"
          disabled={updateMutation.isPending || !canModifySettings}
        >
          {updateMutation.isPending ? "保存中..." : "保存设置"}
        </Button>
      </div>
    </div>
  );
}
