"use client";

import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { MessageBubble } from "./message-bubble";
import type { LocalMessage, Message } from "@/lib/types";
import { useEffect, useRef } from "react";
import { useAppStore } from "@/lib/store";
import { CheckSquare, Repeat, Square } from "lucide-react";

interface MessageListProps {
  messages: LocalMessage[];
  isLoading: boolean;
  onEdit: (message: Message) => void;
  onDelete: (messageId: string) => void;
  onRevert: (messageId: string) => void;
  editingMessageId: string | null;
}

export function MessageList(
  { messages, isLoading, onEdit, onDelete, onRevert, editingMessageId }:
    MessageListProps,
) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const selectAllMessages = useAppStore((state) => state.selectAllMessages);
  const clearMessageSelection = useAppStore((state) =>
    state.clearMessageSelection
  );
  const invertMessageSelection = useAppStore((state) =>
    state.invertMessageSelection
  );

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const messageIds = messages.map((m) => m.id);
  const hasSelection = selectedMessages.size > 0;
  const allSelected = selectedMessages.size === messages.length &&
    messages.length > 0;

  if (isLoading) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <p className="text-sm text-muted-foreground">加载消息中...</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col overflow-hidden h-full">
      <div className="flex items-center justify-between border-b bg-muted/30 px-4 py-2">
        <div className="text-sm text-muted-foreground">
          {hasSelection
            ? `已选择 ${selectedMessages.size} 条消息`
            : `共 ${messages.length} 条消息`}
        </div>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => (allSelected
              ? clearMessageSelection()
              : selectAllMessages(messageIds))}
            title={allSelected ? "取消全选" : "全选"}
          >
            {allSelected
              ? <Square className="mr-1 h-3 w-3" />
              : <CheckSquare className="mr-1 h-3 w-3" />}
            {allSelected ? "取消全选" : "全选"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => invertMessageSelection(messageIds)}
            disabled={messages.length === 0}
            title="反选"
          >
            <Repeat className="mr-1 h-3 w-3" />
            反选
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1 h-0 p-4" ref={scrollRef}>
        <div className="space-y-2 mx-2 mt-2">
          {messages.length === 0
            ? (
              <div className="flex h-full items-center justify-center">
                <p className="text-sm text-muted-foreground">
                  暂无消息，开始对话吧
                </p>
              </div>
            )
            : (
              messages.map((message, index) => (
                <MessageBubble
                  key={message.id}
                  message={message}
                  messageNumber={index + 1}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  onRevert={onRevert}
                  showCheckbox={true}
                  isEditing={editingMessageId === message.id}
                />
              ))
            )}
        </div>
      </ScrollArea>
    </div>
  );
}
