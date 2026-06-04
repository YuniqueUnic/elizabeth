"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Copy, Edit2, RotateCcw, Trash2 } from "lucide-react";
import type { Message } from "@/lib/types";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { formatDate } from "@/lib/utils/format";
import { formatSingleMessageMarkdown } from "@/lib/utils/message-format";
import { useState } from "react";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { MinimalTiptapViewer } from "./minimal-tiptap-viewer";
import { Badge } from "@/components/ui/badge";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { useTranslations } from "next-intl";


interface MessageBubbleProps {
  message: Message;
  messageNumber: number;
  onEdit: (message: Message) => void;
  onDelete: (messageId: string) => void;
  onRevert: (messageId: string) => void;
  showCheckbox?: boolean;
  isEditing?: boolean;
}

export function MessageBubble(
  {
    message,
    messageNumber,
    onEdit,
    onDelete,
    onRevert,
    showCheckbox,
    isEditing = false,
  }: MessageBubbleProps,
) {
  const t = useTranslations("room");
  const tCommon = useTranslations("common");
  const [isHovered, setIsHovered] = useState(false);
  const [copied, setCopied] = useState(false);
  const { toast } = useToast();
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const toggleMessageSelection = useAppStore((state) =>
    state.toggleMessageSelection
  );
  const includeMetadataInCopy = useAppStore(
    (state) => state.includeMetadataInCopy,
  );
  const useHeti = useAppStore((state) => state.useHeti);
  const { can } = useRoomPermissions();


  const isSelected = selectedMessages.has(message.id);

  const handleCopy = async () => {
    const textToCopy = formatSingleMessageMarkdown(message, messageNumber, {
      includeMetadata: includeMetadataInCopy,
      tHeader: (p) => tCommon("messageHeader", p),
      tUser: (p) => tCommon("messageUser", p),
      tAnonymous: tCommon("messageAnonymous"),
      tTime: (p) => tCommon("messageTime", p),
    });

    try {
      await copyTextToClipboard(textToCopy);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      toast({
        title: t("messageBubble.copied"),
        description: t("messageBubble.copiedDescription"),
      });
    } catch {
      toast({
        title: tCommon("copyFailed"),
        description: tCommon("copyFailedDescription"),
        variant: "destructive",
      });
    }
  };

  const handleDelete = () => {
    // 只调用 onDelete，不显示 toast
    // toast 会在实际删除操作完成后由父组件显示
    onDelete(message.id);
  };

  const handleMouseEnter = () => {
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    setIsHovered(false);
  };

  return (
    <div
      className={`group relative rounded-lg border bg-card p-4 transition-colors hover:bg-accent/50 min-w-0 ${
        isSelected ? "ring-2 ring-primary" : ""
      } ${message.isPendingDelete ? "opacity-50" : ""}`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      data-testid={`message-item-${message.id}`}
    >
      {showCheckbox && (
        <div className="absolute left-2 top-2">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => toggleMessageSelection(message.id)}
            data-testid={`message-checkbox-${message.id}`}
          />
        </div>
      )}

      {/* Message Content */}
      <div
        className={`message-content overflow-hidden wrap-break-word ${
          showCheckbox ? "ml-6" : ""
        } ${message.isPendingDelete ? "line-through" : ""}`}
        data-testid={`message-content-${message.id}`}
      >
        <MinimalTiptapViewer
          content={message.content}
          className={useHeti ? "heti" : ""}
          onFileClick={(fileId) => useAppStore.getState().setPreviewFileId(fileId)}
        />
      </div>

      {/* Message Meta */}
      <div
        className={`mt-2 flex items-center justify-between text-xs text-muted-foreground ${
          showCheckbox ? "ml-6" : ""
        }`}
        data-testid={`message-meta-${message.id}`}
      >
        <span>
          #{messageNumber} · {message.user || t("messageBubble.anonymous")}
        </span>
        <span title={new Date(message.timestamp).toLocaleString("zh-CN")}>
          {formatDate(message.timestamp)}
        </span>
      </div>

      {/* Status Badges */}
      <div className="absolute left-8 top-0 -translate-y-1/2 transform">
        <div className="flex gap-1">
          {isEditing && (
            <Badge
              variant="default"
              className="text-xs bg-blue-500"
              data-testid={`message-editing-badge-${message.id}`}
            >
              {t("messageBubble.editing")}
            </Badge>
          )}
          {message.isNew && !isEditing && (
            <Badge
              variant="outline"
              className="text-xs"
              data-testid={`message-unsaved-badge-${message.id}`}
            >
              {t("messageBubble.unsaved")}
            </Badge>
          )}
          {message.isDirty && !isEditing && (
            <Badge
              variant="outline"
              className="text-xs"
              data-testid={`message-edited-badge-${message.id}`}
            >
              {t("messageBubble.edited")}
            </Badge>
          )}
        </div>
      </div>

      {/* Action Buttons */}
      {isHovered && (
        <div className="absolute right-2 top-2 flex gap-1 rounded-md border bg-background p-1 shadow-sm">
          {message.isPendingDelete
            ? (
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6"
                title={t("messageBubble.revertDelete")}
                onClick={() => onRevert(message.id)}
                disabled={!can.delete}
              >
                <RotateCcw className="h-3 w-3" />
              </Button>
            )
            : (
              <>
                {message.isDirty && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    title={t("messageBubble.revertEdit")}
                    onClick={() => onRevert(message.id)}
                    disabled={!can.edit}
                  >
                    <RotateCcw className="h-3 w-3" />
                  </Button>
                )}
                {
                  /* The original code had a `can("edit")` check here, but `can` is not defined.
                   Assuming it's a placeholder for a permission check or similar.
                   For now, I'll remove it as it's not part of the requested edit. */
                }
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => onEdit(message)}
                  title={t("messageBubble.edit")}
                  disabled={!can.edit}
                >
                  <Edit2 className="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={handleCopy}
                  title={copied ? t("messageBubble.copied") : t("messageBubble.copy")}
                >
                  <Copy className="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-destructive hover:text-destructive"
                  onClick={handleDelete}
                  title={t("messageBubble.delete")}
                  disabled={!can.delete}
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </>
            )}
        </div>
      )}
    </div>
  );
}
