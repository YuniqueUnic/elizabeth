"use client";

import type { Editor } from "@tiptap/react";
import React, { useRef } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ToolbarButton } from "./toolbar-button";
import {
  Bold,
  Italic,
  Strikethrough,
  Code,
  List,
  ListOrdered,
  Quote,
  Undo,
  Redo,
  Heading1,
  Heading2,
  Heading3,
  Paperclip,
  FileCode,
} from "lucide-react";

interface EditorToolbarProps {
  editor: Editor | null;
  onUpload: (files: File[]) => void;
  disabled?: boolean;
  isSourceMode: boolean;
  onToggleSourceMode: () => void;
  onAction: (format: string) => void;
  className?: string;
}

export function EditorToolbar({
  editor,
  onUpload,
  disabled,
  isSourceMode,
  onToggleSourceMode,
  onAction,
  className,
}: EditorToolbarProps) {
  const t = useTranslations("room.messageInput");
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (!editor) return null;

  const handleAction = (format: string, run: () => void) => {
    if (isSourceMode) {
      onAction(format);
    } else {
      run();
    }
  };

  return (
    <div
      className={cn(
        "flex flex-nowrap sm:flex-wrap items-center gap-1 p-1 bg-transparent border-0 overflow-x-auto sm:overflow-x-visible scrollbar-none",
        className
      )}
      style={{
        scrollbarWidth: "none",
        msOverflowStyle: "none",
      }}
    >
      {/* 文本样式组 */}
      <div className="flex items-center gap-0.5 shrink-0">
        <ToolbarButton
          onClick={() => handleAction("bold", () => editor.chain().focus().toggleBold().run())}
          active={!isSourceMode && editor.isActive("bold")}
          title={t("toolbarBold")}
          disabled={disabled}
        >
          <Bold className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("italic", () => editor.chain().focus().toggleItalic().run())}
          active={!isSourceMode && editor.isActive("italic")}
          title={t("toolbarItalic")}
          disabled={disabled}
        >
          <Italic className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("strike", () => editor.chain().focus().toggleStrike().run())}
          active={!isSourceMode && editor.isActive("strike")}
          title={t("toolbarStrike")}
          disabled={disabled}
        >
          <Strikethrough className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("code", () => editor.chain().focus().toggleCode().run())}
          active={!isSourceMode && editor.isActive("code")}
          title={t("toolbarCode")}
          disabled={disabled}
        >
          <Code className="h-3.5 w-3.5" />
        </ToolbarButton>
      </div>

      <div className="w-px h-4 bg-border/60 mx-1 shrink-0" />

      {/* 标题级别组 */}
      <div className="flex items-center gap-0.5 shrink-0">
        <ToolbarButton
          onClick={() => handleAction("heading-1", () => editor.chain().focus().toggleHeading({ level: 1 }).run())}
          active={!isSourceMode && editor.isActive("heading", { level: 1 })}
          title={t("toolbarH1")}
          disabled={disabled}
        >
          <Heading1 className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("heading-2", () => editor.chain().focus().toggleHeading({ level: 2 }).run())}
          active={!isSourceMode && editor.isActive("heading", { level: 2 })}
          title={t("toolbarH2")}
          disabled={disabled}
        >
          <Heading2 className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("heading-3", () => editor.chain().focus().toggleHeading({ level: 3 }).run())}
          active={!isSourceMode && editor.isActive("heading", { level: 3 })}
          title={t("toolbarH3")}
          disabled={disabled}
        >
          <Heading3 className="h-3.5 w-3.5" />
        </ToolbarButton>
      </div>

      <div className="w-px h-4 bg-border/60 mx-1 shrink-0" />

      {/* 结构组织组 */}
      <div className="flex items-center gap-0.5 shrink-0">
        <ToolbarButton
          onClick={() => handleAction("bulletList", () => editor.chain().focus().toggleBulletList().run())}
          active={!isSourceMode && editor.isActive("bulletList")}
          title={t("toolbarBulletList")}
          disabled={disabled}
        >
          <List className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("orderedList", () => editor.chain().focus().toggleOrderedList().run())}
          active={!isSourceMode && editor.isActive("orderedList")}
          title={t("toolbarOrderedList")}
          disabled={disabled}
        >
          <ListOrdered className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("blockquote", () => editor.chain().focus().toggleBlockquote().run())}
          active={!isSourceMode && editor.isActive("blockquote")}
          title={t("toolbarBlockquote")}
          disabled={disabled}
        >
          <Quote className="h-3.5 w-3.5" />
        </ToolbarButton>
      </div>

      <div className="w-px h-4 bg-border/60 mx-1 shrink-0" />

      {/* 撤销重做 & 资源附件 */}
      <div className="flex items-center gap-0.5 shrink-0">
        <ToolbarButton
          onClick={() => handleAction("undo", () => editor.chain().focus().undo().run())}
          title={t("toolbarUndo")}
          disabled={disabled}
        >
          <Undo className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => handleAction("redo", () => editor.chain().focus().redo().run())}
          title={t("toolbarRedo")}
          disabled={disabled}
        >
          <Redo className="h-3.5 w-3.5" />
        </ToolbarButton>

        <ToolbarButton
          onClick={() => fileInputRef.current?.click()}
          title={t("toolbarUpload")}
          disabled={disabled}
        >
          <Paperclip className="h-3.5 w-3.5" />
        </ToolbarButton>
      </div>

      <div className="flex-grow" />

      {/* 预览切换模式 */}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onToggleSourceMode}
        disabled={disabled}
        className={cn(
          "h-7 w-7 p-0 rounded-md transition-all duration-200 text-muted-foreground hover:bg-muted/65 hover:text-foreground shrink-0",
          isSourceMode && "bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary"
        )}
        title={isSourceMode ? t("toolbarPreviewMode") : t("toolbarSourceMode")}
      >
        <FileCode className="h-3.5 w-3.5" />
      </Button>

      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => {
          const files = Array.from(e.target.files || []);
          if (files.length > 0) {
            onUpload(files);
          }
          e.target.value = "";
        }}
      />
    </div>
  );
}
