"use client";

import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Underline from "@tiptap/extension-underline";
import Image from "@tiptap/extension-image";
import Link from "@tiptap/extension-link";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { Markdown } from "@tiptap/markdown";
import { common, createLowlight } from "lowlight";
import { useAppStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { useEffect } from "react";

const lowlight = createLowlight(common);

interface MinimalTiptapViewerProps {
  content: string;
  className?: string;
}

export function MinimalTiptapViewer({ content, className }: MinimalTiptapViewerProps) {
  const messageFontSize = useAppStore((state) => state.messageFontSize);

  const editor = useEditor({
    editable: false, // 只读模式
    extensions: [
      StarterKit.configure({
        codeBlock: false,
      }),
      CodeBlockLowlight.configure({
        lowlight,
        defaultLanguage: "plaintext",
      }),
      Markdown.configure({
        markedOptions: {
          gfm: true,
          breaks: true,
        },
      }),
      Underline,
      Image.configure({
        HTMLAttributes: {
          class: "max-w-full rounded-md border border-border",
        },
      }),
      Link.configure({
        openOnClick: false, // 我们通过点击事件处理链接跳转
        HTMLAttributes: {
          class: "text-primary hover:underline cursor-pointer",
          target: "_blank",
          rel: "noopener noreferrer",
        },
      }),
    ],
    content: content,
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
    if (editor) {
        // 尝试解析 markdown 并设置内容
        try {
            const json = editor.storage.markdown?.manager?.parse(content);
            if (json) {
                // 如果内容实际上没有变化，setContent 可能会被优化掉，但这里我们无法轻易比较
                // 所以我们依赖 Tiptap 内部的 diff
                editor.commands.setContent(json);
            } else {
                editor.commands.setContent(content);
            }
        } catch (e) {
            console.error("Failed to parse markdown in viewer:", e);
            editor.commands.setContent(content);
        }
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
