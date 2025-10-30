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
import { useCallback, useEffect, useState } from "react";
import type { LocalMessage, Message } from "@/lib/types";
import { useToast } from "@/hooks/use-toast";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
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
import { Button } from "@/components/ui/button";

export function MiddleColumn() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const messages = useAppStore((state) => state.messages);
  const setMessages = useAppStore((state) => state.setMessages);
  const addMessage = useAppStore((state) => state.addMessage);
  const updateMessageContent = useAppStore(
    (state) => state.updateMessageContent,
  );
  const markMessageForDeletion = useAppStore(
    (state) => state.markMessageForDeletion,
  );
  const revertMessageChanges = useAppStore(
    (state) => state.revertMessageChanges,
  );
  const showDeleteConfirmation = useAppStore(
    (state) => state.showDeleteConfirmation,
  );
  const setShowDeleteConfirmation = useAppStore(
    (state) => state.setShowDeleteConfirmation,
  );

  const queryClient = useQueryClient();
  const [editingMessage, setEditingMessage] = useState<Message | null>(null);
  const [deleteCandidateId, setDeleteCandidateId] = useState<string | null>(
    null,
  );
  const { toast } = useToast();
  const { can } = useRoomPermissions();
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchMessages = async () => {
      if (currentRoomId) {
        setIsLoading(true);
        try {
          const fetchedMessages = await getMessages(currentRoomId);
          setMessages(fetchedMessages);
        } catch (error) {
          console.error("Failed to fetch messages:", error);
          toast({
            title: "加载失败",
            description: "无法加载消息，请刷新重试",
            variant: "destructive",
          });
        } finally {
          setIsLoading(false);
        }
      }
    };

    fetchMessages();
  }, [currentRoomId, setMessages, toast]);

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
        updateMessageContent(editingMessage.id, content);
        setEditingMessage(null);
      } else {
        addMessage(content);
      }
    },
    [editingMessage, updateMessageContent, addMessage],
  );

  const handleEdit = useCallback((message: Message) => {
    setEditingMessage(message);
  }, []);

  const handleCancelEdit = () => {
    setEditingMessage(null);
  };

  const handleDelete = (messageId: string) => {
    if (showDeleteConfirmation) {
      setDeleteCandidateId(messageId);
    } else {
      markMessageForDeletion(messageId);
    }
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
            onRevert={revertMessageChanges}
            editingMessageId={editingMessage?.id || null}
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
      <AlertDialog
        open={deleteCandidateId !== null}
        onOpenChange={(open) => !open && setDeleteCandidateId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>你确定要删除这条消息吗？</AlertDialogTitle>
            <AlertDialogDescription>
              这个操作将会被记录，直到你点击保存按钮。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <div className="flex w-full items-center justify-between">
              <Button
                variant="outline"
                onClick={() => {
                  if (deleteCandidateId) {
                    markMessageForDeletion(deleteCandidateId);
                  }
                  setShowDeleteConfirmation(false);
                  setDeleteCandidateId(null);
                }}
              >
                确认/并不再提示
              </Button>
              <div className="flex gap-2">
                <AlertDialogCancel>取消</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    if (deleteCandidateId) {
                      markMessageForDeletion(deleteCandidateId);
                    }
                  }}
                >
                  确认
                </AlertDialogAction>
              </div>
            </div>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </main>
  );
}
