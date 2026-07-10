"use client";

import { MessageList } from "@/components/chat/message-list";
import { MessageInput } from "@/components/chat/message-input";
import { useQuery } from "@tanstack/react-query";
import { getRoomDetails } from "@/api/roomService";
import { useAppStore } from "@/lib/store";
import { useCallback, useState } from "react";
import { useTranslations } from "next-intl";
import type { Message } from "@/lib/types";
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
import { useRoomMessages } from "@/lib/hooks/use-room-messages";

export function MiddleColumn() {
  const t = useTranslations("room");
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const {
    messages,
    isInitialLoading,
    isLoadingOlder,
    hasMore,
    loadOlder,
  } = useRoomMessages(currentRoomId);
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

  const [deleteCandidateId, setDeleteCandidateId] = useState<string | null>(
    null,
  );
  const { data: roomDetails } = useQuery({
    queryKey: ["room", currentRoomId],
    queryFn: () => getRoomDetails(currentRoomId),
    staleTime: 1000,
    enabled: !!currentRoomId,
  });
  const { can } = useRoomPermissions(roomDetails?.permissions);

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

  const handleDelete = useCallback((messageId: string) => {
    showDeleteConfirmation
      ? setDeleteCandidateId(messageId)
      : markMessageForDeletion(messageId);
  }, [markMessageForDeletion, showDeleteConfirmation]);

  return (
    <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
      <Group orientation="vertical">
        {/* 消息列表面板 */}
        <Panel defaultSize={70} minSize={30}>
          <MessageList
            messages={messages}
            isLoading={isInitialLoading}
            isLoadingOlder={isLoadingOlder}
            hasMore={hasMore}
            onLoadOlder={loadOlder}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onRevert={revertMessageChanges}
            editingMessageId={composerEditingMessageId}
            canEdit={can.edit}
            canDelete={can.delete}
          />
        </Panel>

        {/* 可拖动分割线 */}
        <Separator className="h-1 bg-border hover:bg-primary/50 transition-colors cursor-row-resize relative group">
          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-10 h-1 bg-muted-foreground/30 rounded-full group-hover:bg-primary/70 transition-colors" />
        </Separator>

        {/* 编辑器面板 */}
        <Panel defaultSize={30} minSize={20}>
          <MessageInput
            onSend={handleSend}
            editingMessage={composerEditingMessageId
              ? messages.find((m) => m.id === composerEditingMessageId) ?? null
              : null}
            onCancelEdit={handleCancelEdit}
            isLoading={false}
          />
        </Panel>
      </Group>
      <AlertDialog
        open={deleteCandidateId !== null}
        onOpenChange={(open) => !open && setDeleteCandidateId(null)}
      >
        <AlertDialogContent data-testid="delete-confirm-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>{t("chat.deleteConfirmTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("chat.deleteConfirmDescription")}
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
                {t("chat.confirmAndDisable")}
              </Button>
              <div className="flex gap-2">
                <AlertDialogCancel>{t("chat.cancel")}</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    if (deleteCandidateId) {
                      markMessageForDeletion(deleteCandidateId);
                    }
                  }}
                >
                  {t("chat.confirm")}
                </AlertDialogAction>
              </div>
            </div>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </main>
  );
}
