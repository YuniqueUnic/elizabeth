"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Maximize2, Send, X } from "lucide-react";
import { useCallback, useState } from "react";
import { useAppStore } from "@/lib/store";
import type { Message } from "@/lib/types";
import { MinimalTiptapEditor } from "./minimal-tiptap-editor";
import { useTranslations } from "next-intl";

interface MessageInputProps {
  onSend: (content: string) => void;
  editingMessage: Message | null;
  onCancelEdit: () => void;
  isLoading: boolean;
}

function getSendableContent(markdown: string): string {
  const decodedEntities = markdown
    .replace(/&#x([0-9a-fA-F]+);/g, (_match, hex: string) => {
      const codePoint = Number.parseInt(hex, 16);
      return Number.isFinite(codePoint) ? String.fromCodePoint(codePoint) : "";
    })
    .replace(/&#([0-9]+);/g, (_match, decimal: string) => {
      const codePoint = Number.parseInt(decimal, 10);
      return Number.isFinite(codePoint) ? String.fromCodePoint(codePoint) : "";
    })
    .replace(/&nbsp;/gi, " ");

  const withoutComments = decodedEntities.replace(/<!--[\s\S]*?-->/g, "");
  const normalized = withoutComments
    .replace(/[\p{Cc}\p{Cf}]/gu, (match) => {
      // 保留换行符、回车符和制表符
      if (match === "\n" || match === "\r" || match === "\t") return match;
      return "";
    })
    .replace(/ /g, " ");
  return normalized.trim();
}

export function MessageInput(
  { onSend, editingMessage, onCancelEdit, isLoading }: MessageInputProps,
) {
  const t = useTranslations("room");
  const [isExpanded, setIsExpanded] = useState(false);
  const sendOnEnter = useAppStore((state) => state.sendOnEnter);
  const content = useAppStore((state) => state.composerContent);
  const setContent = useAppStore((state) => state.setComposerContent);
  // const diffMarkdown = editingMessage?.originalContent ?? editingMessage?.content;

  const handleSend = useCallback(() => {
    const sendable = getSendableContent(content);
    if (!sendable || isLoading) return;
    onSend(sendable);
    setIsExpanded(false);
  }, [content, isLoading, onSend]);

  return (
    <>
      <div className="bg-background h-full flex flex-col p-1">
        {/* Editing Banner */}
        {editingMessage && (
          <div className="shrink-0 mb-2 flex items-center justify-between rounded-lg bg-muted/70 px-3 py-1.5 text-xs text-muted-foreground border border-border/40">
            <span>{t("messageInput.editingBanner")}</span>
            <Button
              variant="ghost"
              size="icon"
              className="h-5 w-5 rounded-md"
              onClick={onCancelEdit}
            >
              <X className="h-3 w-3" />
            </Button>
          </div>
        )}

        <div className="flex-grow min-h-0 relative">
          <MinimalTiptapEditor
            value={content}
            onChange={setContent}
            onRequestSend={handleSend}
            disabled={isLoading}
            placeholder={sendOnEnter
              ? t("messageInput.placeholderSendOnEnter")
              : t("messageInput.placeholderSendOnCtrl")}
            sendOnEnter={sendOnEnter}
            toolbarPosition="bottom"
            className="h-full min-h-[120px] bg-card/45 backdrop-blur-sm border border-border/60 shadow-[0_2px_12px_rgba(0,0,0,0.03)] focus-within:shadow-[0_4px_20px_rgba(0,0,0,0.05)] rounded-xl"
            editorClassName="px-4 py-3"
            renderActions={() => (
              <>
                <Button
                  variant="ghost"
                  type="button"
                  size="icon"
                  className="h-7 w-7 rounded-md text-muted-foreground hover:bg-muted/75 hover:text-foreground"
                  onClick={() => setIsExpanded(true)}
                  title={t("messageInput.expandEditor")}
                >
                  <Maximize2 className="h-4 w-4" />
                </Button>
                <Button
                  size="sm"
                  type="button"
                  className="h-7 px-3 rounded-md shadow-sm gap-1 transition-all duration-200"
                  onClick={handleSend}
                  disabled={!getSendableContent(content) || isLoading}
                >
                  <Send className="h-3.5 w-3.5" />
                  <span className="text-xs">{t("messageInput.send")}</span>
                </Button>
              </>
            )}
          />
        </div>
      </div>

      <Dialog open={isExpanded} onOpenChange={setIsExpanded}>
        <DialogContent
          showCloseButton={false}
          className="max-w-none w-screen h-screen sm:h-[80vh] sm:max-w-4xl lg:max-w-5xl sm:w-full p-0 gap-0 flex flex-col sm:rounded-xl rounded-none overflow-hidden bg-card border-border/80 shadow-[0_25px_60px_-15px_rgba(0,0,0,0.3)]"
        >
          {/* Header */}
          <div className="px-4 sm:px-6 py-3 sm:py-4 flex items-center justify-between border-b border-border/60 bg-muted/20">
            <div>
              <DialogTitle className="text-sm font-semibold">{t("messageInput.markdownEditor")}</DialogTitle>
              <p className="text-xs text-muted-foreground mt-0.5">{t("messageInput.markdownEditorSubtitle")}</p>
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 rounded-full text-muted-foreground hover:bg-muted"
              onClick={() => setIsExpanded(false)}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          {/* Editor Body */}
          <div className="flex-1 min-h-0 bg-transparent p-0">
            <MinimalTiptapEditor
              value={content}
              onChange={setContent}
              onRequestSend={handleSend}
              disabled={isLoading}
              placeholder={t("messageInput.placeholderDefault")}
              sendOnEnter={sendOnEnter}
              toolbarPosition="top"
              className="h-full border-0 rounded-none bg-transparent shadow-none focus-within:ring-0 focus-within:border-0"
              editorClassName="p-4 sm:p-6"
            />
          </div>

          {/* Footer */}
          <div className="px-4 sm:px-6 py-3 sm:py-3.5 pb-5 sm:pb-3.5 flex justify-end gap-2 bg-muted/10 border-t border-border/60">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsExpanded(false)}
              className="rounded-lg h-9 px-4 text-xs"
            >
              {t("messageInput.cancel")}
            </Button>
            <Button
              size="sm"
              onClick={handleSend}
              disabled={!getSendableContent(content) || isLoading}
              className="rounded-lg h-9 px-4 text-xs shadow-sm gap-1.5"
            >
              <Send className="h-3.5 w-3.5" />
              {t("messageInput.send")}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
