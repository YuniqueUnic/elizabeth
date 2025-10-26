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
import { getMessages, getRoomDetails } from "@/api/roomService";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { formatDate } from "@/lib/utils/format";

export function TopBar() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const includeMetadataInExport = useAppStore((state) =>
    state.includeMetadataInExport
  );
  const { toast } = useToast();

  const { data: roomDetails } = useQuery({
    queryKey: ["room", currentRoomId],
    queryFn: () => getRoomDetails(currentRoomId),
  });

  const { data: messages = [] } = useQuery({
    queryKey: ["messages", currentRoomId],
    queryFn: () => getMessages(currentRoomId),
  });

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
        if (includeMetadataInExport) {
          const messageNumber = messages.findIndex((msg) => msg.id === m.id) +
            1;
          return `### 消息 #${messageNumber}\n**用户:** ${m.user}\n**时间:** ${
            formatDate(m.timestamp)
          }\n\n${m.content}`;
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
        if (includeMetadataInExport) {
          const messageNumber = messages.findIndex((msg) => msg.id === m.id) +
            1;
          return `### 消息 #${messageNumber}\n**用户:** ${m.user}\n**时间:** ${
            formatDate(m.timestamp)
          }\n\n${m.content}`;
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
        >
          <Save className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="hidden md:flex"
          title="删除"
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
    </header>
  );
}
