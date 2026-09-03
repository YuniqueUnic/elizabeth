"use client";

import { useTranslations } from "next-intl";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { MemberAccessPanel } from "@/components/room/member-access-panel";
import { RoleMatrixEditor } from "@/components/room/role-matrix-editor";

interface RoomPermissionsDialogProps {
  roomName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * 房间成员与权限配置入口：把身份码分发、会话管理与角色矩阵这些
 * 低频、复杂的操作收进单独的对话框，侧边栏只保留常用简单配置。
 */
export function RoomPermissionsDialog({
  roomName,
  open,
  onOpenChange,
}: RoomPermissionsDialogProps) {
  const t = useTranslations("room.members");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[85vh] flex-col gap-4 overflow-hidden sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="members" className="flex min-h-0 flex-1 flex-col gap-4">
          <TabsList className="w-fit">
            <TabsTrigger value="members">{t("tabMembers")}</TabsTrigger>
            <TabsTrigger value="roles">{t("tabRoles")}</TabsTrigger>
          </TabsList>

          <div className="-mx-1 min-h-0 flex-1 overflow-y-auto px-1 pb-1">
            <TabsContent value="members" className="m-0">
              {open && <MemberAccessPanel roomName={roomName} />}
            </TabsContent>
            <TabsContent value="roles" className="m-0">
              {open && <RoleMatrixEditor roomName={roomName} />}
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
