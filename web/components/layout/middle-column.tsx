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
import { Group, Panel, Separator } from "react-resizable-panels";
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
import {
  handleMutationError,
  handleMutationSuccess,
} from "@/lib/utils/mutations";

export function MiddleColumn() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const messages = useAppStore((state) => state.messages);
  const setMessages = useAppStore((state) => state.setMessages);
  const addMessage = useAppStore((state) => state.addMessage);
  const updateMessageContent = useAppStore(
    (state) => state.updateMessageContent,
  );
  const composerEditingMessageId = useAppStore(
    (state) => state.composerEditingMessageId,
  );
  const beginEditMessage = useAppStore((state) => state.beginEditMessage);
  const cancelEditMessage = useAppStore((state) => state.cancelEditMessage);
  const setComposerContent = useAppStore((state) => state.setComposerContent);
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
  const [deleteCandidateId, setDeleteCandidateId] = useState<string | null>(
    null,
  );
  const { toast } = useToast();
  const { can } = useRoomPermissions();
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchMessages = async () => {
      if (!currentRoomId) return;
      setIsLoading(true);
      try {
        const fetchedMessages = await getMessages(currentRoomId);
        setMessages(fetchedMessages);
      } catch (error) {
        handleMutationError(error, toast, {
          description: "无法加载消息，请刷新重试",
        });
      } finally {
        setIsLoading(false);
      }
    };

    fetchMessages();
  }, [currentRoomId, setMessages, toast]);

  const createOptimisticMessage = (content: string): Message => ({
    id: `temp-${Date.now()}`,
    content,
    timestamp: new Date().toISOString(),
    isOwn: true,
  });

  const postMutation = useMutation({
    mutationFn: (content: string) => postMessage(currentRoomId, content),
    onMutate: async (content: string) => {
      await queryClient.cancelQueries({
        queryKey: ["messages", currentRoomId],
      });

      const previousMessages = queryClient.getQueryData([
        "messages",
        currentRoomId,
      ]);

      const optimisticMessage = createOptimisticMessage(content);
      queryClient.setQueryData(
        ["messages", currentRoomId],
        (old: Message[] = []) => [...old, optimisticMessage],
      );

      return { previousMessages, optimisticMessage };
    },
    onError: (error, content, context) => {
      if (context?.previousMessages) {
        queryClient.setQueryData(
          ["messages", currentRoomId],
          context.previousMessages,
        );
      }
      handleMutationError(error, toast, {
        description: "无法发送消息，请重试",
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

      handleMutationSuccess(toast, {
        title: "消息已发送",
      });
    },
    onSettled: () => {
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
      cancelEditMessage();
      toast({
        title: "消息已更新",
        description: "您的消息已成功更新",
      });
    },
    onError: () => {
      handleMutationError(null, toast, {
        description: "无法更新消息，请重试",
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (messageId: string) => deleteMessage(currentRoomId, messageId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] });
      handleMutationSuccess(toast, { title: "消息已删除" });
    },
    onError: (error) => {
      handleMutationError(error, toast, {
        description: "无法删除消息，请重试",
      });
    },
  });

  const handleSend = useCallback(
    (content: string) => {
      if (composerEditingMessageId) {
        updateMessageContent(composerEditingMessageId, content);
        cancelEditMessage();
        return;
      }

      addMessage(content);
      setComposerContent("");
    },
    [
      composerEditingMessageId,
      updateMessageContent,
      addMessage,
      cancelEditMessage,
      setComposerContent,
    ],
  );

  const handleEdit = useCallback(
    (message: Message) => beginEditMessage(message.id),
    [beginEditMessage],
  );

  const handleCancelEdit = () => {
    cancelEditMessage();
  };

  const handleDelete = (messageId: string) => {
    showDeleteConfirmation
      ? setDeleteCandidateId(messageId)
      : markMessageForDeletion(messageId);
  };

  return (
    <main className="flex flex-1 flex-col overflow-hidden">
      <Group orientation="vertical">
        {/* 消息列表面板 */}
        <Panel defaultSize={70} minSize={30}>
          <MessageList
            messages={messages}
            isLoading={isLoading}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onRevert={revertMessageChanges}
            editingMessageId={composerEditingMessageId}
          />
        </Panel>

        {/* 可拖动分割线 */}
        <Separator className="h-1 bg-border hover:bg-primary/50 transition-colors cursor-row-resize relative group">
          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-10 h-1 bg-muted-foreground/30 rounded-full group-hover:bg-primary/70 transition-colors" />
        </Separator>

        {/* 编辑器面板 */}
        {can.edit && (
          <Panel defaultSize={30} minSize={20}>
            <MessageInput
              onSend={handleSend}
              editingMessage={composerEditingMessageId
                ? messages.find((m) => m.id === composerEditingMessageId) ?? null
                : null}
              onCancelEdit={handleCancelEdit}
              isLoading={postMutation.isPending || updateMutation.isPending}
            />
          </Panel>
        )}
      </Group>
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
