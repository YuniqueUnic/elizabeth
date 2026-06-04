"use client";

import { Button } from "@/components/ui/button";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { LanguageSwitcher } from "@/components/language-switcher";
import { SettingsDialog } from "@/components/settings-dialog";
import { HelpDialog } from "@/components/help-dialog";
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
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import {
  formatMessagesMarkdown,
  downloadMarkdown,
} from "@/lib/utils/message-format";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslations } from "next-intl";
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
import {
  handleMutationError,
  handleMutationSuccess,
} from "@/lib/utils/mutations";

export function TopBar() {
  const t = useTranslations("common");
  const queryClient = useQueryClient();
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const messages = useAppStore((state) => state.messages);
  const hasUnsavedChanges = useAppStore((state) => state.hasUnsavedChanges);
  const isSaving = useAppStore((state) => state.isSaving);
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
  const [isDeleteConfirmationOpen, setIsDeleteConfirmationOpen] = useState(
    false,
  );

  const { data: roomDetails } = useQuery({
    queryKey: ["room", currentRoomId],
    queryFn: () => getRoomDetails(currentRoomId),
  });

  const handleSaveChanges = async () => {
    try {
      await saveMessages();
      handleMutationSuccess(toast, {
        title: t("saveSuccess"),
        description: t("allChangesSaved"),
      });
    } catch (error) {
      handleMutationError(error, toast, {
        description: t("saveFailed"),
      });
    }
  };

  const handleDeleteMessages = () => {
    if (selectedMessages.size === 0) {
      toast({
        title: t("noMessagesSelected"),
        description: t("selectMessagesToDelete"),
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
        title: t("noMessagesSelected"),
        description: t("selectMessagesToCopy"),
        variant: "destructive",
      });
      return;
    }

    try {
      const text = formatMessagesMarkdown(messages, selectedMessages, {
        includeMetadata: includeMetadataInCopy,
        tHeader: (p) => t("messageHeader", p),
        tUser: (p) => t("messageUser", p),
        tAnonymous: t("messageAnonymous"),
        tTime: (p) => t("messageTime", p),
      });
      await copyTextToClipboard(text);
      toast({
        title: t("copied"),
        description: t("copiedMessagesToClipboard", { count: selectedMessages.size }),
      });
    } catch {
      toast({
        title: t("copyFailed"),
        description: t("copyFailedDescription"),
        variant: "destructive",
      });
    }
  };

  const handleDownloadMessages = () => {
    if (selectedMessages.size === 0) {
      toast({
        title: t("noMessagesSelected"),
        description: t("selectMessagesToExport"),
        variant: "destructive",
      });
      return;
    }

    try {
      const text = formatMessagesMarkdown(messages, selectedMessages, {
        includeMetadata: includeMetadataInDownload,
        tHeader: (p) => t("messageHeader", p),
        tUser: (p) => t("messageUser", p),
        tAnonymous: t("messageAnonymous"),
        tTime: (p) => t("messageTime", p),
      });
      downloadMarkdown(text);
      toast({
        title: t("exportSuccess"),
        description: t("exportedMessagesAsMarkdown", { count: selectedMessages.size }),
      });
    } catch {
      toast({
        title: t("exportFailed"),
        description: t("exportFailedDescription"),
        variant: "destructive",
      });
    }
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
          <div className="hidden sm:block ml-4 text-sm text-muted-foreground truncate max-w-37.5 md:max-w-none">
            {t("roomUsage", { used: (roomDetails.currentSize / (1024 * 1024)).toFixed(1), total: (roomDetails.maxSize / (1024 * 1024)).toFixed(1) })}
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div className="flex items-center gap-1 md:gap-2">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 md:h-10 md:w-10"
          title={t("copySelectedMessages")}
          onClick={handleCopyMessages}
          disabled={selectedMessages.size === 0}
          data-testid="copy-messages-btn"
        >
          <Copy className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 md:h-10 md:w-10"
          title={t("downloadExportSelectedMessages")}
          onClick={handleDownloadMessages}
          disabled={selectedMessages.size === 0}
          data-testid="download-messages-btn"
        >
          <Download className="h-4 w-4" />
        </Button>
        <Button
          variant={hasUnsavedChanges() ? "default" : "ghost"}
          size="icon"
          className={`h-8 w-8 md:h-10 md:w-10 ${
            hasUnsavedChanges() && !isSaving
              ? "bg-primary text-primary-foreground hover:bg-primary/90"
              : ""
          }`}
          title={isSaving ? t("saving") : t("save")}
          onClick={handleSaveChanges}
          disabled={!hasUnsavedChanges() || isSaving}
          data-testid="save-messages-btn"
        >
          <Save className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 md:h-10 md:w-10"
          title={t("delete")}
          onClick={handleDeleteMessages}
          disabled={selectedMessages.size === 0}
          data-testid="delete-messages-btn"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
        <HelpDialog>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 md:h-10 md:w-10"
            title={t("help")}
            data-testid="help-btn"
          >
            <HelpCircle className="h-4 w-4" />
          </Button>
        </HelpDialog>

        <div className="mx-1 md:mx-2 h-6 w-px bg-border" />

        <SettingsDialog>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 md:h-10 md:w-10"
            title={t("settings")}
            data-testid="settings-btn"
          >
            <Settings className="h-4 w-4" />
          </Button>
        </SettingsDialog>

        <LanguageSwitcher />
        <ThemeSwitcher />
      </div>
      <AlertDialog
        open={isDeleteConfirmationOpen}
        onOpenChange={setIsDeleteConfirmationOpen}
      >
        <AlertDialogContent data-testid="delete-confirm-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("confirmDeleteTitle", { count: selectedMessages.size })}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("confirmDeleteDescription")}
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
                {t("confirmAndDontAsk")}
              </Button>
              <div className="flex gap-2">
                <AlertDialogCancel>{t("cancel")}</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    selectedMessages.forEach((messageId) => {
                      markMessageForDeletion(messageId);
                    });
                  }}
                >
                  {t("confirm")}
                </AlertDialogAction>
              </div>
            </div>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </header>
  );
}
