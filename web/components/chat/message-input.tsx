"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Maximize2, Send, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useAppStore } from "@/lib/store";
import type { Message } from "@/lib/types";
import { EnhancedMarkdownEditor } from "./enhanced-markdown-editor";

interface MessageInputProps {
  onSend: (content: string) => void;
  editingMessage: Message | null;
  onCancelEdit: () => void;
  isLoading: boolean;
}

export function MessageInput(
  { onSend, editingMessage, onCancelEdit, isLoading }: MessageInputProps,
) {
  const [content, setContent] = useState("");
  const [isExpanded, setIsExpanded] = useState(false);
  const sendOnEnter = useAppStore((state) => state.sendOnEnter);

  useEffect(() => {
    if (editingMessage) {
      setContent(editingMessage.content);
    } else {
      setContent("");
    }
  }, [editingMessage]);

  const handleSend = () => {
    if (content.trim() && !isLoading) {
      onSend(content.trim());
      setContent("");
      setIsExpanded(false);
    }
  };

  return (
    <>
      <div className="bg-background p-4 h-full flex flex-col">
        {/* Editing Banner */}
        {editingMessage && (
          <div className="mb-2 flex items-center justify-between rounded-md bg-muted px-3 py-2 text-sm">
            <span>正在编辑消息</span>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={onCancelEdit}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}

        <div className="flex-1 flex flex-col gap-2 min-h-0 overflow-hidden">
          <div className="flex-1 min-h-0 overflow-hidden">
            <EnhancedMarkdownEditor
              value={content}
              onChange={setContent}
              placeholder={sendOnEnter
                ? "输入消息... (Enter 发送, Shift+Enter 换行)"
                : "输入消息... (Ctrl/Cmd+Enter 发送)"}
              height="100%"
              showPreview={false}
            />
          </div>

          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsExpanded(true)}
              title="展开编辑器"
            >
              <Maximize2 className="mr-2 h-4 w-4" />
              展开编辑器
            </Button>
            <Button
              size="sm"
              onClick={handleSend}
              disabled={!content.trim() || isLoading}
            >
              <Send className="mr-2 h-4 w-4" />
              发送
            </Button>
          </div>
        </div>
      </div>

      <Dialog open={isExpanded} onOpenChange={setIsExpanded}>
        <DialogContent className="max-w-none w-screen h-screen sm:h-[90vh] sm:max-w-4xl lg:max-w-6xl sm:w-full p-0 sm:p-6 gap-0 flex flex-col sm:rounded-lg rounded-none">
          <DialogTitle className="sr-only">Markdown 编辑器</DialogTitle>
          <div className="flex-1 overflow-hidden px-4 py-3 sm:px-0 sm:py-0 min-h-0">
            <div className="h-full">
              <EnhancedMarkdownEditor
                value={content}
                onChange={setContent}
                placeholder="输入消息..."
                showPreview={true}
              />
            </div>
          </div>

          <div className="flex justify-end gap-2 px-4 pb-4 sm:px-0 sm:pb-0 pt-3 border-t">
            <Button variant="outline" onClick={() => setIsExpanded(false)}>
              取消
            </Button>
            <Button
              onClick={handleSend}
              disabled={!content.trim() || isLoading}
            >
              <Send className="mr-2 h-4 w-4" />
              发送
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
