"use client";

import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { MessageBubble } from "./message-bubble";
import type { LocalMessage, Message } from "@/lib/types";
import { useEffect, useRef, useState, useCallback } from "react";
import { useAppStore } from "@/lib/store";
import { CheckSquare, Repeat, Square, ArrowDown } from "lucide-react";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/utils";

const SCROLL_THRESHOLD = 100;

interface MessageListProps {
  messages: LocalMessage[];
  isLoading: boolean;
  isLoadingOlder: boolean;
  hasMore: boolean;
  onLoadOlder: () => Promise<void>;
  onEdit: (message: Message) => void;
  onDelete: (messageId: string) => void;
  onRevert: (messageId: string) => void;
  editingMessageId: string | null;
  canEdit: boolean;
  canDelete: boolean;
}

export function MessageList(
  {
    messages,
    isLoading,
    isLoadingOlder,
    hasMore,
    onLoadOlder,
    onEdit,
    onDelete,
    onRevert,
    editingMessageId,
    canEdit,
    canDelete,
  }:
    MessageListProps,
) {
  const t = useTranslations("room");
  const scrollRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const selectAllMessages = useAppStore((state) => state.selectAllMessages);
  const clearMessageSelection = useAppStore((state) =>
    state.clearMessageSelection
  );
  const invertMessageSelection = useAppStore((state) =>
    state.invertMessageSelection
  );
  const autoScroll = useAppStore((state) => state.autoScroll);
  const [isNearBottom, setIsNearBottom] = useState(true);
  const [showJumpButton, setShowJumpButton] = useState(false);
  const loadingOlderRef = useRef(false);

  const loadOlderWithAnchor = useCallback(async () => {
    const viewport = viewportRef.current;
    if (
      !viewport || loadingOlderRef.current || isLoadingOlder || !hasMore ||
      messages.length === 0
    ) return;

    loadingOlderRef.current = true;
    const previousHeight = viewport.scrollHeight;
    const previousTop = viewport.scrollTop;
    try {
      await onLoadOlder();
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const currentViewport = viewportRef.current;
          if (!currentViewport) return;
          currentViewport.scrollTop = previousTop +
            (currentViewport.scrollHeight - previousHeight);
        });
      });
    } finally {
      loadingOlderRef.current = false;
    }
  }, [hasMore, isLoadingOlder, messages.length, onLoadOlder]);

  const checkNearBottom = useCallback((el: HTMLDivElement | null) => {
    if (!el) return true;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    const near = distanceFromBottom < SCROLL_THRESHOLD;
    setIsNearBottom(near);
    setShowJumpButton(!near && messages.length > 0);
    return near;
  }, [messages.length]);

  // Find the ScrollArea viewport element and attach native scroll listener
  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;
    const viewport = container.querySelector(
      "[data-radix-scroll-area-viewport]",
    ) as HTMLDivElement | null;
    if (!viewport) return;
    viewportRef.current = viewport;

    const onScroll = () => {
      checkNearBottom(viewport);
      if (viewport.scrollTop <= SCROLL_THRESHOLD) {
        void loadOlderWithAnchor();
      }
    };
    viewport.addEventListener("scroll", onScroll, { passive: true });

    // Initial check
    checkNearBottom(viewport);

    return () => {
      viewport.removeEventListener("scroll", onScroll);
      viewportRef.current = null;
    };
  }, [checkNearBottom, loadOlderWithAnchor]);

  // Auto-scroll when new messages arrive
  useEffect(() => {
    if (autoScroll && isNearBottom) {
      const el = viewportRef.current;
      if (el) {
        requestAnimationFrame(() => {
          el.scrollTop = el.scrollHeight;
        });
      }
    }
  }, [messages, autoScroll, isNearBottom]);

  // Scroll to bottom on initial load
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    if (autoScroll) {
      el.scrollTop = el.scrollHeight;
    } else {
      // When auto-scroll is off, start at the top so user can browse freely
      el.scrollTop = 0;
    }
  }, [autoScroll]);

  const scrollToBottom = () => {
    const el = viewportRef.current;
    if (el) {
      el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
    }
  };

  const messageIds = messages.map((m) => m.id);
  const hasSelection = selectedMessages.size > 0;
  const allSelected = selectedMessages.size === messages.length &&
    messages.length > 0;

  if (isLoading) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <p className="text-sm text-muted-foreground">{t("messageList.loading")}</p>
      </div>
    );
  }

  return (
    <div className="flex h-full min-w-0 flex-col overflow-hidden">
      <div
        className={cn(
          "items-center justify-between border-b bg-muted/30 px-4 py-2",
          hasSelection ? "flex" : "hidden sm:flex",
        )}
        data-testid="message-selection-toolbar"
      >
        <div className="text-sm text-muted-foreground">
          {hasSelection
            ? t("messageList.selectedCount", { count: selectedMessages.size })
            : t("messageList.loadedCount", { count: messages.length })}
        </div>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => (allSelected
              ? clearMessageSelection()
              : selectAllMessages(messageIds))}
            title={allSelected ? t("messageList.deselectAll") : t("messageList.selectAll")}
          >
            {allSelected
              ? <Square className="mr-1 h-3 w-3" />
              : <CheckSquare className="mr-1 h-3 w-3" />}
            {allSelected ? t("messageList.deselectAll") : t("messageList.selectAll")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => invertMessageSelection(messageIds)}
            disabled={messages.length === 0}
            title={t("messageList.invertSelection")}
          >
            <Repeat className="mr-1 h-3 w-3" />
            {t("messageList.invertSelection")}
          </Button>
        </div>
      </div>

      <div
        className="relative h-0 min-w-0 flex-1"
        data-testid="message-list-scroll"
      >
        <ScrollArea
          className="h-full min-w-0 p-4"
          ref={scrollRef}
        >
          <div className="mx-2 mt-2 min-w-0 space-y-2">
            {(hasMore || isLoadingOlder) && (
              <div className="flex min-h-8 items-center justify-center py-1">
                {isLoadingOlder
                  ? (
                    <p
                      className="text-xs text-muted-foreground"
                      role="status"
                      aria-live="polite"
                    >
                      {t("messageList.loadingOlder")}
                    </p>
                  )
                  : (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => void loadOlderWithAnchor()}
                    >
                      {t("messageList.loadOlder")}
                    </Button>
                  )}
              </div>
            )}
            {messages.length === 0
              ? (
                <div className="flex h-full items-center justify-center">
                  <p className="text-sm text-muted-foreground">
                    {t("messageList.empty")}
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
                    canEdit={canEdit}
                    canDelete={canDelete}
                  />
                ))
              )}
          </div>
        </ScrollArea>

        {showJumpButton && (
          <Button
            variant="secondary"
            size="sm"
            className="absolute bottom-4 right-4 shadow-md"
            onClick={scrollToBottom}
          >
            <ArrowDown className="mr-1 h-3 w-3" />
            {t("messageList.scrollToLatest")}
          </Button>
        )}
      </div>
    </div>
  );
}
