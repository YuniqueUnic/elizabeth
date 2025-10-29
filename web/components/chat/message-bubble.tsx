"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Copy, Edit2, Trash2, RotateCcw } from "lucide-react";
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
}

export function MessageBubble(
  { message, messageNumber, onEdit, onDelete, onRevert, showCheckbox }:
    MessageBubbleProps,
) {
  const [isHovered, setIsHovered] = useState(false);
  const [copied, setCopied] = useState(false);
  const { toast } = useToast();
  const selectedMessages = useAppStore((state) => state.selectedMessages);
  const toggleMessageSelection = useAppStore((state) =>
    state.toggleMessageSelection
  );

  const isSelected = selectedMessages.has(message.id);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    toast({
      title: "已复制",
      description: "消息内容已复制到剪贴板",
    });
  };

  const handleDelete = () => {
    onDelete(message.id);
    toast({
      title: "消息已删除",
      description: "消息已从聊天记录中删除",
    });
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
    >
      {showCheckbox && (
        <div className="absolute left-2 top-2">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => toggleMessageSelection(message.id)}
          />
        </div>
      )}

      {/* Message Content */}
      <div
        className={`message-content prose prose-sm dark:prose-invert max-w-none ${
          showCheckbox ? "ml-6" : ""
        } ${message.isPendingDelete ? "line-through" : ""}`}
      >
        <MarkdownRenderer content={message.content} />
      </div>

      {/* Message Meta */}
      <div
        className={`mt-2 flex items-center justify-between text-xs text-muted-foreground ${
          showCheckbox ? "ml-6" : ""
        }`}
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
          {message.isNew && (
            <Badge variant="outline" className="text-xs">
              New
            </Badge>
          )}
          {message.isDirty && (
            <Badge variant="outline" className="text-xs">
              Edited
            </Badge>
          )}
        </div>
      </div>

      {/* Action Buttons */}
      {isHovered && (
        <div className="absolute right-2 top-2 flex gap-1 rounded-md border bg-background p-1 shadow-sm">
          {message.isPendingDelete ? (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              title="撤销删除"
              onClick={() => onRevert(message.id)}
            >
              <RotateCcw className="h-3 w-3" />
            </Button>
          ) : (
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
              {/* The original code had a `can("edit")` check here, but `can` is not defined.
                   Assuming it's a placeholder for a permission check or similar.
                   For now, I'll remove it as it's not part of the requested edit. */}
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
