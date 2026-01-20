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
}

export function MinimalTiptapViewer({ content, className }: MinimalTiptapViewerProps) {
  const messageFontSize = useAppStore((state) => state.messageFontSize);

  const editor = useEditor({
    editable: false,
    extensions: [
      // StarterKit 在 v3 已经包含了 Link 和 Underline，所以不需要单独导入
      StarterKit.configure({
        codeBlock: false, // 禁用默认的 codeBlock，使用 CodeBlockLowlight
      }),
      CodeBlockLowlight.configure({
        lowlight,
        defaultLanguage: "plaintext",
      }),
      Markdown,
      ImageAuth.configure({
        HTMLAttributes: {
          // 限制图片默认大小，添加点击放大功能
          class: "max-w-sm max-h-64 object-contain rounded-md border border-border cursor-zoom-in",
        },
      }),
    ],
    content,
    // 关键修复：指定内容类型为 markdown
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
