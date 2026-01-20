"use client";

import { useEffect, useState } from "react";
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
import { getAccessToken } from "@/api/authService";

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
  // { label: "永不过期", value: "never", ms: 0 }, // 暂不提供，未来可能支持
];

// 根据过期时间计算最接近的选项
function getExpiryOptionFromDate(expiresAt: string | null | undefined): string {
  // 如果没有过期时间，默认设置为 1 周（因为已移除"永不过期"选项）
  if (!expiresAt) return "1week";

  // 后端返回的是 NaiveDateTime (UTC 时间，无时区标记)
  // 需要手动添加 'Z' 后缀来表示这是 UTC 时间
  const expiresAtUTC = expiresAt.endsWith("Z") ? expiresAt : expiresAt + "Z";
  const expireTime = new Date(expiresAtUTC).getTime();
  const now = Date.now();
  const diff = expireTime - now;

  // 如果已经过期或即将过期，返回最短的选项
  if (diff <= 0) return "1min";

  // 找到最接近的选项
  let closestOption = EXPIRY_OPTIONS[0];
  let minDiff = Math.abs(diff - closestOption.ms);

  for (const option of EXPIRY_OPTIONS) {
    if (option.ms === 0) continue; // 跳过永不过期选项（如果未来启用）
    const currentDiff = Math.abs(diff - option.ms);
    if (currentDiff < minDiff) {
      minDiff = currentDiff;
      closestOption = option;
    }
  }

  return closestOption.value;
}

export function RoomSettingsForm({ roomDetails }: RoomSettingsFormProps) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { can } = useRoomPermissions();

  const [expiryOption, setExpiryOption] = useState(() =>
    getExpiryOptionFromDate(roomDetails.settings.expiresAt)
  );
  const [password, setPassword] = useState(roomDetails.password || "");
  const [showPassword, setShowPassword] = useState(false);
  const [maxViews, setMaxViews] = useState(roomDetails.settings.maxViews);

  // 当 roomDetails 更新时，同步更新状态
  useEffect(() => {
    setExpiryOption(getExpiryOptionFromDate(roomDetails.settings.expiresAt));
    setPassword(roomDetails.password || "");
    setMaxViews(roomDetails.settings.maxViews);
  }, [roomDetails]);

  // 只有拥有删除权限的用户才能修改房间设置
  const canModifySettings = can.delete;

  const updateMutation = useMutation({
    mutationFn: (settings: {
      password?: string | null;
      expiresAt?: string | null;
      maxViews?: number;
      passwordChanged?: boolean; // Track if password was changed
    }) => updateRoomSettings(currentRoomId, settings),
    onSuccess: async (updatedRoom, variables) => {
      // 方法 1: 直接更新缓存数据，而不是失效缓存
      queryClient.setQueryData(["room", currentRoomId], updatedRoom);

      // 方法 2: 同时失效缓存并立即重新获取
      await queryClient.refetchQueries({
        queryKey: ["room", currentRoomId],
        type: "active",
      });

      // ✅ FIX: If password was changed, automatically refresh JWT with new password
      if (variables.passwordChanged && variables.password) {
        try {
          console.log(
            "[RoomSettingsForm] Password changed, refreshing JWT with new password",
          );
          await getAccessToken(currentRoomId, variables.password);
          console.log("[RoomSettingsForm] JWT refreshed successfully");
        } catch (error) {
          console.error("[RoomSettingsForm] Failed to refresh JWT:", error);
          toast({
            title: "警告",
            description: "密码已更新，但 JWT 刷新失败。请刷新页面重新登录。",
            variant: "destructive",
          });
          return;
        }
      }

      toast({
        title: "设置已保存",
        description: "房间设置已成功更新",
      });
    },
    onError: (error: any) => {
      // ✅ FIX: Provide clearer error messages
      const errorMessage = error?.message || "无法保存房间设置，请重试";
      const isAuthError = error?.status === 401 || error?.status === 403;

      toast({
        title: "保存失败",
        description: isAuthError
          ? "认证失败，请刷新页面重新登录"
          : errorMessage,
        variant: "destructive",
      });
    },
  });

  const handleSave = () => {
    const option = EXPIRY_OPTIONS.find((opt) => opt.value === expiryOption);

    // 计算过期时间，格式化为 NaiveDateTime (YYYY-MM-DDTHH:MM:SS.ffffff)
    // 注意：后端使用 UTC 时间进行比较，所以这里必须发送 UTC 时间
    let expiresAt: string | null = null;
    if (option && option.ms > 0) {
      // 使用 UTC 时间计算过期时间
      const now = new Date();
      const expireDate = new Date(now.getTime() + option.ms);
      // toISOString() 返回 UTC 时间，格式：YYYY-MM-DDTHH:MM:SS.sssZ
      // 去掉末尾的 'Z' 得到 NaiveDateTime 格式
      expiresAt = expireDate.toISOString().replace("Z", "");
    }

    // Detect if password was changed
    // - If unchanged, omit the field to avoid unintended token side effects.
    // - If changed to empty string, backend treats it as "clear password".
    const newPassword = password.trim();
    const oldPassword = roomDetails.password || "";
    const passwordChanged = newPassword !== oldPassword;

    updateMutation.mutate({
      expiresAt: expiresAt ?? undefined,
      password: passwordChanged ? newPassword : undefined,
      maxViews,
      passwordChanged,
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
