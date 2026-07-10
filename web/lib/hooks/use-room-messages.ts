"use client";

import { useCallback, useEffect } from "react";
import { useTranslations } from "next-intl";

import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { handleMutationError } from "@/lib/utils/mutations";

export function useRoomMessages(roomId: string) {
  const t = useTranslations("room");
  const { toast } = useToast();
  const messages = useAppStore((state) => state.messages);
  const initialStatus = useAppStore((state) => state.messageInitialStatus);
  const olderStatus = useAppStore((state) => state.messageOlderStatus);
  const hasMore = useAppStore((state) => state.messageHasMore);
  const ensureMessagesLoaded = useAppStore((state) =>
    state.ensureMessagesLoaded
  );
  const loadOlderMessages = useAppStore((state) => state.loadOlderMessages);

  useEffect(() => {
    void ensureMessagesLoaded(roomId).catch((error) => {
      handleMutationError(error, toast, {
        description: t("chat.loadFailed"),
      });
    });
  }, [ensureMessagesLoaded, roomId, t, toast]);

  const loadOlder = useCallback(async () => {
    try {
      await loadOlderMessages();
    } catch (error) {
      handleMutationError(error, toast, {
        description: t("chat.loadFailed"),
      });
    }
  }, [loadOlderMessages, t, toast]);

  return {
    messages,
    isInitialLoading: messages.length === 0 &&
      (initialStatus === "idle" || initialStatus === "loading"),
    isLoadingOlder: olderStatus === "loading",
    hasMore,
    loadOlder,
  };
}
