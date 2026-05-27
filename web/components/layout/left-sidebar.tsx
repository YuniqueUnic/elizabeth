"use client";

import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, XCircle, Loader2 } from "lucide-react";
import { useAppStore } from "@/lib/store";
import { RoomConfigForm } from "@/components/room/room-config-form";
import { RoomCapacity } from "@/components/room/room-capacity";
import { RoomSharing } from "@/components/room/room-sharing";
import { useQuery } from "@tanstack/react-query";
import { getRoomDetails, deleteRoom } from "@/api/roomService";
import { verifyRoomPassword } from "@/api/authService";

import { clearRoomToken } from "@/lib/utils/api";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useIsMobile } from "@/hooks/use-mobile";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { useToast } from "@/hooks/use-toast";
import { useRouter } from "next/navigation";
import { useState } from "react";
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

export function LeftSidebar() {
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
  const { can } = useRoomPermissions();

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
      setDialogError("请输入密码");
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
      setDialogError("密码验证失败，密码错误或无法连接服务器");
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
        title: "房间已成功关闭并清理",
        description: `房间 ${roomDetails.name} 的数据已彻底清除，原房名已立即释放。`,
      });
      setIsCloseDialogOpen(false);
      router.push("/");
    } catch (err: any) {
      console.error("Failed to delete room:", err);
      toast({
        title: "关闭房间失败",
        description: err.message || "无法删除房间，请稍后重试",
        variant: "destructive",
      });
    } finally {
      setActionLoading(false);
    }
  };

  // Mobile layout: full width, no collapse button
  if (isMobile) {
    return (
      <div className="flex h-full w-full flex-col bg-background">
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">房间设置</h2>
        </div>

        <ScrollArea className="flex-1">
          <div className="space-y-6 p-4">
            {isLoading
              ? (
                <div className="text-center text-sm text-muted-foreground">
                  加载中...
                </div>
              )
              : roomDetails
              ? (
                <>
                  <RoomConfigForm roomDetails={roomDetails} />
                  <RoomSharing
                    key={roomDetails.slug}
                    roomId={roomDetails.slug || roomDetails.name}
                  />
                  <RoomCapacity
                    currentSize={roomDetails.currentSize}
                    maxSize={roomDetails.maxSize}
                  />

                  {/* 关闭房间区域 */}
                  <div className="pt-4 border-t mt-4">
                    <Button
                      variant="destructive"
                      className="w-full justify-center gap-2"
                      disabled={!can.delete}
                      onClick={handleOpenCloseRoom}
                      title={!can.delete ? "您没有删除该房间的管理员权限" : "永久关闭并清空该房间"}
                    >
                      <XCircle className="h-4 w-4" />
                      关闭房间
                    </Button>
                  </div>
                </>
              )
              : null}
          </div>
        </ScrollArea>
      </div>
    );
  }

  // Desktop layout: fixed width with collapse functionality
  if (leftSidebarCollapsed) {
    return (
      <div className="flex w-12 flex-col items-center border-r bg-muted/30 py-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleLeftSidebar}
          title="展开侧边栏"
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    );
  }

  return (
    <>
      <aside className="flex w-80 flex-col border-r bg-muted/30 h-full overflow-hidden">
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">房间控制</h2>
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleLeftSidebar}
            title="收起侧边栏"
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
        </div>

        <ScrollArea className="flex-1 h-0">
          <div className="space-y-6 p-4">
            {isLoading
              ? (
                <div className="text-center text-sm text-muted-foreground">
                  加载中...
                </div>
              )
              : roomDetails
              ? (
                <>
                  <RoomConfigForm roomDetails={roomDetails} />
                  <RoomSharing
                    key={roomDetails.slug}
                    roomId={roomDetails.slug || roomDetails.name}
                  />
                  <RoomCapacity
                    currentSize={roomDetails.currentSize}
                    maxSize={roomDetails.maxSize}
                  />
                </>
              )
              : null}
          </div>
        </ScrollArea>

        {/* 底部关闭房间区域 (固定在最下方) */}
        {roomDetails && (
          <div className="border-t p-4 bg-muted/20 mt-auto shrink-0">
            <Button
              variant="destructive"
              className="w-full justify-center gap-2"
              disabled={!can.delete}
              onClick={handleOpenCloseRoom}
              title={!can.delete ? "您没有删除该房间的管理员权限" : "永久关闭并清空该房间"}
            >
              <XCircle className="h-4 w-4" />
              关闭房间
            </Button>
          </div>
        )}
      </aside>

      {/* 关闭房间的多步确认对话框 */}
      <Dialog open={isCloseDialogOpen} onOpenChange={handleCloseDialog}>
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <XCircle className="h-5 w-5" />
              关闭房间 - {roomDetails?.name}
            </DialogTitle>
            <DialogDescription>
              {step === 1 ? "该房间设有密码保护，需验证密码后方可继续。" : "此操作将永久物理删除房间及其所有数据。"}
            </DialogDescription>
          </DialogHeader>

          {step === 1 && (
            <div className="grid gap-4 py-4">
              <div className="grid gap-2">
                <Label htmlFor="close-room-password">请输入房间密码：</Label>
                <Input
                  id="close-room-password"
                  type="password"
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    setDialogError(null);
                  }}
                  placeholder="房间密码"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      void handleVerifyPassword();
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
                警告：当前房间内存有所有的消息、图片、文档及配置。一旦关闭，所有内容将被立即永久物理清空。
              </p>
              <p className="text-sm text-muted-foreground">
                该房间名将立即被完全释放，任何人都可以重新创建并使用此房间名。您确定要执行此操作吗？
              </p>
            </div>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={handleCloseDialog} disabled={actionLoading}>
              取消
            </Button>
            {step === 1 ? (
              <Button onClick={handleVerifyPassword} disabled={actionLoading}>
                {actionLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    验证中...
                  </>
                ) : (
                  "下一步"
                )}
              </Button>
            ) : (
              <Button variant="destructive" onClick={handleConfirmDelete} disabled={actionLoading}>
                {actionLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    正在关闭房间...
                  </>
                ) : (
                  "确定物理关闭"
                )}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
