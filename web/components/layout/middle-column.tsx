"use client";

import { MessageList } from "@/components/chat/message-list";
import { MessageInput } from "@/components/chat/message-input";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteMessage,
  getMessages,
  postMessage,
  updateMessage,
} from "@/api/roomService";
import { useAppStore } from "@/lib/store";
import { useState } from "react";
import type { Message } from "@/lib/types";
import { useToast } from "@/hooks/use-toast";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";

export function MiddleColumn() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const queryClient = useQueryClient();
  const [editingMessage, setEditingMessage] = useState<Message | null>(null);
  const { toast } = useToast();

  const { data: messages = [], isLoading } = useQuery({
    queryKey: ["messages", currentRoomId],
    queryFn: () => getMessages(currentRoomId),
  });

  const postMutation = useMutation({
    mutationFn: (content: string) => postMessage(currentRoomId, content),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] });
      toast({
        title: "消息已发送",
        description: "您的消息已成功发送",
      });
    },
    onError: () => {
      toast({
        title: "发送失败",
        description: "无法发送消息，请重试",
        variant: "destructive",
      });
    },
  });

  const updateMutation = useMutation({
    mutationFn: (
      { messageId, content }: { messageId: string; content: string },
    ) => updateMessage(messageId, content),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] });
      setEditingMessage(null);
      toast({
        title: "消息已更新",
        description: "您的消息已成功更新",
      });
    },
    onError: () => {
      toast({
        title: "更新失败",
        description: "无法更新消息，请重试",
        variant: "destructive",
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (messageId: string) => deleteMessage(messageId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] });
    },
  });

  const handleSend = (content: string) => {
    if (editingMessage) {
      updateMutation.mutate({ messageId: editingMessage.id, content });
    } else {
      postMutation.mutate(content);
    }
  };

  const handleEdit = (message: Message) => {
    setEditingMessage(message);
  };

  const handleCancelEdit = () => {
    setEditingMessage(null);
  };

  const handleDelete = (messageId: string) => {
    deleteMutation.mutate(messageId);
  };

  return (
    <main className="flex flex-1 flex-col overflow-hidden">
      <PanelGroup direction="vertical">
        {/* 消息列表面板 */}
        <Panel defaultSize={70} minSize={30}>
          <MessageList
            messages={messages}
            isLoading={isLoading}
            onEdit={handleEdit}
            onDelete={handleDelete}
          />
        </Panel>

        {/* 可拖动分割线 */}
        <PanelResizeHandle className="h-1 bg-border hover:bg-primary/50 transition-colors cursor-row-resize relative group">
          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-10 h-1 bg-muted-foreground/30 rounded-full group-hover:bg-primary/70 transition-colors" />
        </PanelResizeHandle>

        {/* 编辑器面板 */}
        <Panel defaultSize={30} minSize={20}>
          <MessageInput
            onSend={handleSend}
            editingMessage={editingMessage}
            onCancelEdit={handleCancelEdit}
            isLoading={postMutation.isPending || updateMutation.isPending}
          />
        </Panel>
      </PanelGroup>
    </main>
  );
}
