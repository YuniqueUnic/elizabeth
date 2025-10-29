"use client";

import { MessageList } from "@/components/chat/message-list";
import { MessageInput } from "@/components/chat/message-input";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteMessage,
  getMessages,
  postMessage,
  updateMessage,
} from "@/api/messageService";
import { useAppStore } from "@/lib/store";
import { useCallback, useState } from "react";
import type { Message } from "@/lib/types";
import { useToast } from "@/hooks/use-toast";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { useRoomPermissions } from "@/hooks/use-room-permissions";

export function MiddleColumn() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const queryClient = useQueryClient();
  const [editingMessage, setEditingMessage] = useState<Message | null>(null);
  const { toast } = useToast();
  const { can } = useRoomPermissions();

  const { data: messages = [], isLoading } = useQuery({
    queryKey: ["messages", currentRoomId],
    queryFn: () => getMessages(currentRoomId),
    refetchInterval: 5000, // 每 5 秒自动刷新一次，保持实时性
    staleTime: 1000, // 1 秒后认为数据过期
  });

  const postMutation = useMutation({
    mutationFn: (content: string) => postMessage(currentRoomId, content),
    onMutate: async (content: string) => {
      // 取消任何进行中的查询
      await queryClient.cancelQueries({
        queryKey: ["messages", currentRoomId],
      });

      // 快照当前的消息列表
      const previousMessages = queryClient.getQueryData([
        "messages",
        currentRoomId,
      ]);

      // 创建一个临时的乐观消息对象
      const optimisticMessage: Message = {
        id: `temp-${Date.now()}`,
        content,
        timestamp: new Date().toISOString(),
        isOwn: true,
      };

      // 乐观更新：立即将新消息添加到列表中
      queryClient.setQueryData(
        ["messages", currentRoomId],
        (old: Message[] = []) => [
          ...old,
          optimisticMessage,
        ],
      );

      // 返回用于回滚的上下文
      return { previousMessages, optimisticMessage };
    },
    onError: (error, content, context) => {
      // 如果出错，回滚到之前的状态
      if (context?.previousMessages) {
        queryClient.setQueryData(
          ["messages", currentRoomId],
          context.previousMessages,
        );
      }

      toast({
        title: "发送失败",
        description: "无法发送消息，请重试",
        variant: "destructive",
      });
    },
    onSuccess: (newMessage, content, context) => {
      queryClient.setQueryData(
        ["messages", currentRoomId],
        (old: Message[] = []) => {
          return old.map((msg) =>
            msg.id === context?.optimisticMessage.id ? newMessage : msg
          );
        },
      );

      toast({
        title: "消息已发送",
        description: "您的消息已成功发送",
      });
    },
    onSettled: () => {
      // 延迟重新获取数据以确保用户能看到乐观更新
      setTimeout(() => {
        queryClient.invalidateQueries({
          queryKey: ["messages", currentRoomId],
        });
      }, 500);
    },
  });

  const updateMutation = useMutation({
    mutationFn: (
      { messageId, content }: { messageId: string; content: string },
    ) => updateMessage(currentRoomId, messageId, content),
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
    mutationFn: (messageId: string) => deleteMessage(currentRoomId, messageId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] });
    },
  });

  const handleSend = useCallback(
    (content: string) => {
      if (editingMessage) {
        updateMutation.mutate({ messageId: editingMessage.id, content });
      } else {
        postMutation.mutate(content);
      }
    },
    [editingMessage, updateMutation, postMutation],
  );

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
        {can.edit && (
          <Panel defaultSize={30} minSize={20}>
            <MessageInput
              onSend={handleSend}
              editingMessage={editingMessage}
              onCancelEdit={handleCancelEdit}
              isLoading={postMutation.isPending || updateMutation.isPending}
            />
          </Panel>
        )}
      </PanelGroup>
    </main>
  );
}
