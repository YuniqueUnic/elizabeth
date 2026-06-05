"use client";

import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
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
        link: false,
      }),
      Link.configure({
        openOnClick: false,
        autolink: false,
        HTMLAttributes: {
          class: "text-primary underline underline-offset-2 cursor-pointer",
        },
        validate: (href) =>
          /^https?:\/\//.test(href) || href.startsWith("/"),
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
          "prose prose-sm dark:prose-invert max-w-none break-words overflow-hidden",
          "focus:outline-none",
          className
        ),
        style: `font-size: ${messageFontSize}px`,
      },
    },
    immediatelyRender: false,
  });

  // DOM-level click interception for file links and images
  const handleClick = useCallback((e: MouseEvent) => {
    if (!onFileClick) return;
    const target = e.target as HTMLElement;

    // 1. Intercept image clicks (<img> elements)
    if (target.tagName === "IMG") {
      const fileId = target.getAttribute("data-file-id");
      if (fileId) {
        e.preventDefault();
        e.stopPropagation();
        onFileClick(fileId);
        return;
      }

      const dataSrc = target.getAttribute("data-src") || target.getAttribute("src");
      if (dataSrc) {
        const match = dataSrc.match(/contents\/(\d+)/);
        if (match) {
          e.preventDefault();
          e.stopPropagation();
          onFileClick(match[1]);
          return;
        }
      }
    }

    // 2. Intercept file links (<a> elements)
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
        className="tiptap-viewer-container overflow-hidden"
        style={{ fontSize: `${messageFontSize}px` }}
    >
        <EditorContent editor={editor} className="tiptap-viewer-content" />
    </div>
  );
}
