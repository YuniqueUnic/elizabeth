"use client";

import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { Markdown } from "@tiptap/markdown";
import { common, createLowlight } from "lowlight";
import { useAppStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { useEffect } from "react";
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
      handleDOMEvents: {
        click: (_view, event) => {
          const target = event.target as HTMLElement;
          const link = target.closest("a");
          if (link) {
            const href = link.getAttribute("href");
            if (href) {
              const match = href.match(/^\/contents\/(\d+)$/);
              if (match && onFileClick) {
                event.preventDefault();
                onFileClick(match[1]);
                return true;
              }
            }
          }
          return false;
        },
      },
    },
    immediatelyRender: false,
  });

  // 当内容变化时更新编辑器
  useEffect(() => {
    if (editor && !editor.isDestroyed) {
      // 关键修复：使用 contentType: 'markdown' 告诉编辑器解析 markdown
      editor.commands.setContent(content, { contentType: "markdown" });
    }
  }, [content, editor]);

  // 当字体大小变化时更新
  useEffect(() => {
    if (editor) {
        // Tiptap 不支持直接动态更新 attributes.style，但 React 会重新渲染组件
        // editorProps 更新可能需要重新实例化或者手动处理 DOM
        // 不过由于我们传递了 key={messageFontSize} 或者依赖 React 的重新渲染机制
        // 实际上 useEditor 的 editorProps 只有在初始化时生效
        // 我们可以直接操作 DOM 元素或者强制重新渲染
    }
  }, [messageFontSize, editor]);

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
