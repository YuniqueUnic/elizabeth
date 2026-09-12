"use client";

import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, Loader2, XCircle } from "lucide-react";
import { useAppStore } from "@/lib/store";
import { RoomConfigForm } from "@/components/room/room-config-form";
import { RoomCapacity } from "@/components/room/room-capacity";
import { RoomSharing } from "@/components/room/room-sharing";
import { IdentityCard } from "@/components/room/identity-card";
import { useQuery } from "@tanstack/react-query";
import { getRoomDetails, deleteRoom } from "@/api/roomService";
import { verifyRoomPassword } from "@/api/authService";

import { clearRoomToken } from "@/lib/utils/api";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useIsMobile } from "@/hooks/use-mobile";
import { useRoomCapabilities } from "@/hooks/use-room-capabilities";
import { useToast } from "@/hooks/use-toast";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useTranslations } from "next-intl";
import { isPermissionDeniedError } from "@/lib/utils/mutations";
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
import type { RoomDetails } from "@/lib/types";

/** 侧边栏正文：只保留高频简单配置；成员与权限等复杂操作在身份卡入口的对话框内。 */
function SidebarBody({
  roomDetails,
  isLoading,
}: {
  roomDetails?: RoomDetails;
  isLoading: boolean;
}) {
  const t = useTranslations("room");
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const { can } = useRoomCapabilities();

  if (isLoading) {
    return (
      <div className="p-4 text-center text-sm text-muted-foreground">
        {t("sidebar.loading")}
      </div>
    );
  }
  if (!roomDetails) return null;

  return (
    <div className="space-y-6 p-4">
      <IdentityCard roomName={currentRoomId} />

      {can.share && (
        <RoomSharing
          key={roomDetails.slug}
          roomId={roomDetails.slug || roomDetails.name}
          canShare={can.share}
        />
      )}

      {can.settings && <RoomConfigForm roomDetails={roomDetails} />}

      <RoomCapacity
        currentSize={roomDetails.currentSize}
        maxSize={roomDetails.maxSize}
      />
    </div>
  );
}

export function LeftSidebar() {
  const t = useTranslations("room");
  const { leftSidebarCollapsed, toggleLeftSidebar, currentRoomId } =
    useAppStore();
  const isMobile = useIsMobile();

  const { data: roomDetails, isLoading } = useQuery({
    queryKey: ["room", currentRoomId],
    queryFn: () => getRoomDetails(currentRoomId),
    staleTime: 1000, // 1 秒后认为数据过期
    enabled: !!currentRoomId, // 只在有房间 ID 时启用查询
  });

  const { toast } = useToast();
  const router = useRouter();
  const { can } = useRoomCapabilities();

  const [isCloseDialogOpen, setIsCloseDialogOpen] = useState(false);
  const [password, setPassword] = useState("");
  const [step, setStep] = useState(1);
  const [actionLoading, setActionLoading] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  const handleOpenCloseRoom = () => {
    if (!roomDetails) return;
    setDialogError(null);
    setPassword("");
    if (roomDetails.settings?.passwordProtected) {
      setStep(1);
    } else {
      setStep(2);
    }
    setIsCloseDialogOpen(true);
  };

  const handleCloseDialog = () => {
    if (actionLoading) return;
    setIsCloseDialogOpen(false);
  };

  const handleVerifyPassword = async () => {
    if (!roomDetails) return;
    if (!password.trim()) {
      setDialogError(t("closeRoom.enterPassword"));
      return;
    }
    setActionLoading(true);
    setDialogError(null);
    try {
      // 使用专用的密码验证函数，强制走密码校验路径，不使用已缓存的 token
      // 避免持有有效 token 的用户跳过密码验证直接进入下一步
      await verifyRoomPassword(roomDetails.slug || roomDetails.name, password);
      setStep(2);
    } catch (err: any) {
      console.error("Verification failed:", err);
      setDialogError(t("closeRoom.verifyFailed"));
    } finally {
      setActionLoading(false);
    }
  };


  const handleConfirmDelete = async () => {
    if (!roomDetails) return;
    setActionLoading(true);
    try {
      const roomSlugOrName = roomDetails.slug || roomDetails.name;
      await deleteRoom(roomSlugOrName);
      clearRoomToken(roomSlugOrName);

      toast({
        title: t("closeRoom.successTitle"),
        description: t("closeRoom.successDescription", { roomName: roomDetails.name }),
      });
      setIsCloseDialogOpen(false);
      router.push("/");
    } catch (err: any) {
      console.error("Failed to delete room:", err);
      toast({
        title: isPermissionDeniedError(err)
          ? t("permissionDenied.title")
          : t("closeRoom.failTitle"),
        description: isPermissionDeniedError(err)
          ? t("permissionDenied.closeRoom")
          : err.message || t("closeRoom.failDescription"),
        variant: "destructive",
      });
    } finally {
      setActionLoading(false);
    }
  };

  const closeRoomButton = (
    <Button
      variant="destructive"
      className="w-full justify-center gap-2"
      disabled={!can.delete}
      onClick={handleOpenCloseRoom}
      title={!can.delete ? t("closeRoom.noPermissionTooltip") : t("closeRoom.tooltip")}
    >
      <XCircle className="h-4 w-4" />
      {t("closeRoom.button")}
    </Button>
  );

  // Mobile layout: full width, no collapse button
  if (isMobile) {
    return (
      <div className="flex h-full w-full flex-col bg-background">
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">{t("sidebar.roomControl")}</h2>
        </div>

        <ScrollArea className="flex-1">
          <SidebarBody roomDetails={roomDetails} isLoading={isLoading} />
          {roomDetails && (
            <div className="border-t p-4">
              {closeRoomButton}
            </div>
          )}
        </ScrollArea>

        <CloseRoomDialog
          isOpen={isCloseDialogOpen}
          onClose={handleCloseDialog}
          step={step}
          password={password}
          setPassword={setPassword}
          dialogError={dialogError}
          setDialogError={setDialogError}
          actionLoading={actionLoading}
          onVerify={handleVerifyPassword}
          onConfirmDelete={handleConfirmDelete}
          roomName={roomDetails?.name ?? ""}
        />
      </div>
    );
  }

  // Desktop layout: fixed width with collapse functionality
  if (leftSidebarCollapsed) {
    return (
      <div
        className="flex w-12 shrink-0 flex-col items-center border-r bg-muted/30 py-4"
        data-testid="left-sidebar-collapsed-rail"
      >
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleLeftSidebar}
          title={t("sidebar.expandSidebar")}
          data-testid="left-sidebar-expand"
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    );
  }

  return (
    <>
      <aside
        className="flex h-full w-80 shrink-0 flex-col overflow-hidden border-r bg-muted/30"
        data-testid="left-sidebar"
      >
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">{t("sidebar.roomControl")}</h2>
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleLeftSidebar}
            title={t("sidebar.collapseSidebar")}
            data-testid="left-sidebar-collapse"
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
        </div>

        <ScrollArea className="flex-1 h-0">
          <SidebarBody roomDetails={roomDetails} isLoading={isLoading} />
        </ScrollArea>

        {/* 底部关闭房间区域 (固定在最下方) */}
        {roomDetails && (
          <div className="mt-auto shrink-0 border-t bg-muted/20 p-4">
            {closeRoomButton}
          </div>
        )}
      </aside>

      <CloseRoomDialog
        isOpen={isCloseDialogOpen}
        onClose={handleCloseDialog}
        step={step}
        password={password}
        setPassword={setPassword}
        dialogError={dialogError}
        setDialogError={setDialogError}
        actionLoading={actionLoading}
        onVerify={handleVerifyPassword}
        onConfirmDelete={handleConfirmDelete}
        roomName={roomDetails?.name ?? ""}
      />
    </>
  );
}

/** 关闭房间的多步确认对话框；桌面与移动共用。 */
function CloseRoomDialog({
  isOpen,
  onClose,
  step,
  password,
  setPassword,
  dialogError,
  setDialogError,
  actionLoading,
  onVerify,
  onConfirmDelete,
  roomName,
}: {
  isOpen: boolean;
  onClose: () => void;
  step: number;
  password: string;
  setPassword: (value: string) => void;
  dialogError: string | null;
  setDialogError: (value: string | null) => void;
  actionLoading: boolean;
  onVerify: () => Promise<void>;
  onConfirmDelete: () => Promise<void>;
  roomName: string;
}) {
  const t = useTranslations("room");
  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <XCircle className="h-5 w-5" />
            {t("closeRoom.title", { roomName })}
          </DialogTitle>
          <DialogDescription>
            {step === 1 ? t("closeRoom.passwordRequired") : t("closeRoom.destructiveWarning")}
          </DialogDescription>
        </DialogHeader>

        {step === 1 && (
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="close-room-password">{t("closeRoom.passwordLabel")}</Label>
              <Input
                id="close-room-password"
                type="password"
                value={password}
                onChange={(e) => {
                  setPassword(e.target.value);
                  setDialogError(null);
                }}
                placeholder={t("closeRoom.passwordPlaceholder")}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    void onVerify();
                  }
                }}
              />
              {dialogError && (
                <p className="text-sm font-medium text-destructive">{dialogError}</p>
              )}
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="py-4 space-y-3">
            <p className="text-sm font-semibold text-destructive">
              {t("closeRoom.warningPermanent")}
            </p>
            <p className="text-sm text-muted-foreground">
              {t("closeRoom.warningRelease")}
            </p>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={actionLoading}>
            {t("closeRoom.cancel")}
          </Button>
          {step === 1 ? (
            <Button onClick={() => void onVerify()} disabled={actionLoading}>
              {actionLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("closeRoom.verifying")}
                </>
              ) : (
                t("closeRoom.nextStep")
              )}
            </Button>
          ) : (
            <Button variant="destructive" onClick={() => void onConfirmDelete()} disabled={actionLoading}>
              {actionLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("closeRoom.closingRoom")}
                </>
              ) : (
                t("closeRoom.confirmPhysicalClose")
              )}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
