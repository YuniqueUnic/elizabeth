"use client";

import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { Markdown } from "@tiptap/markdown";
import { common, createLowlight } from "lowlight";
import { useAppStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { useEffect, useCallback } from "react";
import { ImageAuth } from "./tiptap-extensions/image-auth";

const lowlight = createLowlight(common);

interface MinimalTiptapViewerProps {
  content: string;
  className?: string;
  onFileClick?: (fileId: string) => void;
}

export function MinimalTiptapViewer({ content, className, onFileClick }: MinimalTiptapViewerProps) {
  const messageFontSize = useAppStore((state) => state.messageFontSize);

  const editor = useEditor({
    editable: false,
    extensions: [
      StarterKit.configure({
        codeBlock: false,
      }),
      CodeBlockLowlight.configure({
        lowlight,
        defaultLanguage: "plaintext",
      }),
      Markdown,
      ImageAuth.configure({
        HTMLAttributes: {
          class: "max-w-sm max-h-64 object-contain rounded-md border border-border cursor-zoom-in",
        },
      }),
    ],
    content,
    contentType: "markdown",
    editorProps: {
      attributes: {
        class: cn(
          "prose prose-sm dark:prose-invert max-w-none",
          "focus:outline-none",
          className
        ),
        style: `font-size: ${messageFontSize}px`,
      },
    },
    immediatelyRender: false,
  });

  // DOM-level click interception for file links
  const handleClick = useCallback((e: MouseEvent) => {
    if (!onFileClick) return;
    const target = e.target as HTMLElement;
    const link = target.closest("a");
    if (link) {
      const href = link.getAttribute("href");
      if (href) {
        const match = href.match(/^\/contents\/(\d+)$/);
        if (match) {
          e.preventDefault();
          e.stopPropagation();
          onFileClick(match[1]);
        }
      }
    }
  }, [onFileClick]);

  useEffect(() => {
    const el = editor?.view.dom;
    if (!el) return;
    el.addEventListener("click", handleClick, true);
    return () => el.removeEventListener("click", handleClick, true);
  }, [editor, handleClick]);

  // Update content when it changes
  useEffect(() => {
    if (editor && !editor.isDestroyed) {
      editor.commands.setContent(content, { contentType: "markdown" });
    }
  }, [content, editor]);

  if (!editor) {
    return null;
  }

  return (
    <div
        className="tiptap-viewer-container"
        style={{ fontSize: `${messageFontSize}px` }}
    >
        <EditorContent editor={editor} className="tiptap-viewer-content" />
    </div>
  );
}
