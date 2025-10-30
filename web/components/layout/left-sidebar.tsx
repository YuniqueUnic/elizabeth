"use client";

import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useAppStore } from "@/lib/store";
import { RoomSettingsForm } from "@/components/room/room-settings-form";
import { RoomPermissions } from "@/components/room/room-permissions";
import { RoomCapacity } from "@/components/room/room-capacity";
import { RoomSharing } from "@/components/room/room-sharing";
import { useQuery } from "@tanstack/react-query";
import { getRoomDetails } from "@/api/roomService";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useIsMobile } from "@/hooks/use-mobile";

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
                  <RoomSettingsForm roomDetails={roomDetails} />
                  <RoomPermissions permissions={roomDetails.permissions} />
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
    <aside className="flex w-80 flex-col border-r bg-muted/30">
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
                <RoomSettingsForm roomDetails={roomDetails} />
                <RoomPermissions permissions={roomDetails.permissions} />
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
    </aside>
  );
}
