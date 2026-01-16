"use client";

import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Underline from "@tiptap/extension-underline";
import Link from "@tiptap/extension-link";
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
}

export function MinimalTiptapViewer({ content, className }: MinimalTiptapViewerProps) {
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
      Underline,
      ImageAuth.configure({
        HTMLAttributes: {
          class: "max-w-full rounded-md border border-border",
        },
      }),
      Link.configure({
        openOnClick: false,
        HTMLAttributes: {
          class: "text-primary hover:underline cursor-pointer",
          target: "_blank",
          rel: "noopener noreferrer",
        },
      }),
    ],
    content, // 直接传递 markdown 内容
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

  // 当内容变化时更新编辑器
  useEffect(() => {
    if (editor && !editor.isDestroyed) {
      // 使用 setContent 更新内容，Markdown 扩展会自动解析
      editor.commands.setContent(content);
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
