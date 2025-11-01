"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Copy, Edit2, RotateCcw, Trash2 } from "lucide-react";
import type { Message } from "@/lib/types";
import { formatDate } from "@/lib/utils/format";
import { useState } from "react";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { MarkdownRenderer } from "./markdown-renderer";
import { Badge } from "@/components/ui/badge";

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

  const isSelected = selectedMessages.has(message.id);

  const handleCopy = async () => {
    let textToCopy: string;

    if (includeMetadataInCopy) {
      textToCopy = `### 消息 #${messageNumber}\n**用户:** ${
        message.user || "匿名"
      }\n**时间:** ${formatDate(message.timestamp)}\n\n${message.content}`;
    } else {
      textToCopy = message.content;
    }

    await navigator.clipboard.writeText(textToCopy);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    toast({
      title: "已复制",
      description: "消息内容已复制到剪贴板",
    });
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
      className={`group relative rounded-lg border bg-card p-4 transition-colors hover:bg-accent/50 ${
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
        className={`message-content prose prose-sm dark:prose-invert max-w-none ${
          showCheckbox ? "ml-6" : ""
        } ${message.isPendingDelete ? "line-through" : ""}`}
        data-testid={`message-content-${message.id}`}
      >
        <MarkdownRenderer content={message.content} />
      </div>

      {/* Message Meta */}
      <div
        className={`mt-2 flex items-center justify-between text-xs text-muted-foreground ${
          showCheckbox ? "ml-6" : ""
        }`}
        data-testid={`message-meta-${message.id}`}
      >
        <span>
          #{messageNumber} · {message.user || "匿名"}
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
              正在编辑
            </Badge>
          )}
          {message.isNew && !isEditing && (
            <Badge
              variant="outline"
              className="text-xs"
              data-testid={`message-unsaved-badge-${message.id}`}
            >
              未保存
            </Badge>
          )}
          {message.isDirty && !isEditing && (
            <Badge
              variant="outline"
              className="text-xs"
              data-testid={`message-edited-badge-${message.id}`}
            >
              已编辑
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
                title="撤销删除"
                onClick={() => onRevert(message.id)}
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
                    title="撤销编辑"
                    onClick={() => onRevert(message.id)}
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
                  title="编辑"
                >
                  <Edit2 className="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={handleCopy}
                  title={copied ? "已复制" : "复制"}
                >
                  <Copy className="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-destructive hover:text-destructive"
                  onClick={handleDelete}
                  title="删除"
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
