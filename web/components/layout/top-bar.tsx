"use client";

import { Button } from "@/components/ui/button";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { SettingsDialog } from "@/components/settings-dialog";
import {
  Copy,
  Download,
  HelpCircle,
  Save,
  Settings,
  Trash2,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { getRoomDetails } from "@/api/roomService";
import { deleteMessages, getMessages } from "@/api/messageService";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { formatDate } from "@/lib/utils/format";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

export function TopBar() {
  const queryClient = useQueryClient();
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const messages = useAppStore((state) => state.messages);
  const hasUnsavedChanges = useAppStore((state) => state.hasUnsavedChanges);
  const saveMessages = useAppStore((state) => state.saveMessages);
  const markMessageForDeletion = useAppStore(
    (state) => state.markMessageForDeletion,
  );
  const showDeleteConfirmation = useAppStore(
    (state) => state.showDeleteConfirmation,
  );
  const setShowDeleteConfirmation = useAppStore(
    (state) => state.setShowDeleteConfirmation,
  );
  const includeMetadataInCopy = useAppStore(
    (state) => state.includeMetadataInCopy,
  );
  const includeMetadataInDownload = useAppStore(
    (state) => state.includeMetadataInDownload,
  );
  const { toast } = useToast();
  const [
    isDeleteConfirmationOpen,
    setIsDeleteConfirmationOpen,
  ] = useState(false);

  const { data: roomDetails } = useQuery({
    queryKey: ["room", currentRoomId],
    queryFn: () => getRoomDetails(currentRoomId),
  });

  const handleSaveChanges = async () => {
    try {
      await saveMessages();
      toast({
        title: "保存成功",
        description: "所有更改已成功保存",
      });
    } catch (error) {
      toast({
        title: "保存失败",
        description: "无法保存更改，请重试",
        variant: "destructive",
      });
    }
  };

  const handleDeleteMessages = () => {
    if (selectedMessages.size === 0) {
      toast({
        title: "未选择消息",
        description: "请先选择要删除的消息",
        variant: "destructive",
      });
      return;
    }

    if (showDeleteConfirmation) {
      setIsDeleteConfirmationOpen(true);
    } else {
      selectedMessages.forEach((messageId) => {
        markMessageForDeletion(messageId);
      });
    }
  };

  const handleCopyMessages = async () => {
    if (selectedMessages.size === 0) {
      toast({
        title: "未选择消息",
        description: "请先选择要复制的消息",
        variant: "destructive",
      });
      return;
    }

    const selectedMessagesList = messages
      .filter((m) => selectedMessages.has(m.id))
      .map((m, index) => {
        if (includeMetadataInCopy) {
          const messageNumber = messages.findIndex((msg) => msg.id === m.id) +
            1;
          return `### 消息 #${messageNumber}\n**用户:** ${
            m.user || "匿名"
          }\n**时间:** ${formatDate(m.timestamp)}\n\n${m.content}`;
        }
        return m.content;
      })
      .join("\n\n---\n\n");

    await navigator.clipboard.writeText(selectedMessagesList);
    toast({
      title: "已复制",
      description: `已复制 ${selectedMessages.size} 条消息到剪贴板`,
    });
  };

  const handleDownloadMessages = () => {
    if (selectedMessages.size === 0) {
      toast({
        title: "未选择消息",
        description: "请先选择要导出的消息",
        variant: "destructive",
      });
      return;
    }

    const selectedMessagesList = messages
      .filter((m) => selectedMessages.has(m.id))
      .map((m, index) => {
        if (includeMetadataInDownload) {
          const messageNumber = messages.findIndex((msg) => msg.id === m.id) +
            1;
          return `### 消息 #${messageNumber}\n**用户:** ${
            m.user || "匿名"
          }\n**时间:** ${formatDate(m.timestamp)}\n\n${m.content}`;
        }
        return m.content;
      })
      .join("\n\n---\n\n");

    const blob = new Blob([selectedMessagesList], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `messages-${new Date().toISOString().split("T")[0]}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    toast({
      title: "导出成功",
      description: `已导出 ${selectedMessages.size} 条消息为 Markdown 文件`,
    });
  };

  return (
    <header className="flex h-14 items-center justify-between border-b bg-background px-4">
      {/* Logo */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground font-semibold">
            E
          </div>
          <span className="text-lg font-semibold">Elizabeth</span>
        </div>

        {/* Room Status */}
        {roomDetails && (
          <div className="hidden sm:block ml-4 text-sm text-muted-foreground truncate max-w-[150px] md:max-w-none">
            房间占用：{roomDetails.currentSize.toFixed(1)} /{" "}
            {roomDetails.maxSize} MB
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div className="flex items-center gap-1 md:gap-2">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 md:h-10 md:w-10"
          title="复制选中消息"
          onClick={handleCopyMessages}
          disabled={selectedMessages.size === 0}
        >
          <Copy className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 md:h-10 md:w-10"
          title="下载导出选中消息"
          onClick={handleDownloadMessages}
          disabled={selectedMessages.size === 0}
        >
          <Download className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="hidden md:flex"
          title="保存"
          onClick={handleSaveChanges}
          disabled={!hasUnsavedChanges()}
        >
          <Save className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="hidden md:flex"
          title="删除"
          onClick={handleDeleteMessages}
          disabled={selectedMessages.size === 0}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="hidden md:flex"
          title="帮助"
        >
          <HelpCircle className="h-4 w-4" />
        </Button>

        <div className="mx-1 md:mx-2 h-6 w-px bg-border" />

        <SettingsDialog>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 md:h-10 md:w-10"
            title="设置"
          >
            <Settings className="h-4 w-4" />
          </Button>
        </SettingsDialog>

        <ThemeSwitcher />
      </div>
      <AlertDialog
        open={isDeleteConfirmationOpen}
        onOpenChange={setIsDeleteConfirmationOpen}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              你确定要删除选中的 {selectedMessages.size} 条消息吗？
            </AlertDialogTitle>
            <AlertDialogDescription>
              这个操作将会被记录，直到你点击保存按钮。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <div className="flex w-full items-center justify-between">
              <Button
                variant="outline"
                onClick={() => {
                  selectedMessages.forEach((messageId) => {
                    markMessageForDeletion(messageId);
                  });
                  setShowDeleteConfirmation(false);
                  setIsDeleteConfirmationOpen(false);
                }}
              >
                确认/并不再提示
              </Button>
              <div className="flex gap-2">
                <AlertDialogCancel>取消</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    selectedMessages.forEach((messageId) => {
                      markMessageForDeletion(messageId);
                    });
                  }}
                >
                  确认
                </AlertDialogAction>
              </div>
            </div>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </header>
  );
}
