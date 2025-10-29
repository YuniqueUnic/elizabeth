"use client";

import { useEffect, useState } from "react";
import { useAppStore } from "@/lib/store";
import dynamic from "next/dynamic";
import "@uiw/react-md-editor/markdown-editor.css";
import "@uiw/react-markdown-preview/markdown.css";

const MDEditor = dynamic(() => import("@uiw/react-md-editor"), { ssr: false });

interface EnhancedMarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  height?: number | string;
  showPreview?: boolean;
}

export function EnhancedMarkdownEditor({
  value,
  onChange,
  placeholder,
  height = 120,
  showPreview = false,
}: EnhancedMarkdownEditorProps) {
  const theme = useAppStore((state) => state.theme);
  const sendOnEnter = useAppStore((state) => state.sendOnEnter);
  const [mounted, setMounted] = useState(false);
  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">(
    "light",
  );

  useEffect(() => {
    setMounted(true);
  }, []);

  // 解析主题：如果是 system，则根据系统偏好设置
  useEffect(() => {
    if (theme === "system") {
      const mediaQuery = window.matchMedia(
        "(prefers-color-scheme: dark)",
      );
      const updateTheme = () => {
        setResolvedTheme(mediaQuery.matches ? "dark" : "light");
      };

      updateTheme();
      mediaQuery.addEventListener("change", updateTheme);

      return () => mediaQuery.removeEventListener("change", updateTheme);
    } else {
      setResolvedTheme(theme);
    }
  }, [theme]);

  // 如果 showPreview 为 true（全屏模式），强制使用 100% 高度
  const editorHeight = showPreview ? "100%" : height;

  if (!mounted) {
    return (
      <div
        className="flex items-center justify-center border rounded-md bg-muted/50"
        style={{ height: `${height}px` }}
      >
        <span className="text-sm text-muted-foreground">
          加载编辑器...
        </span>
      </div>
    );
  }

  return (
    <div
      data-color-mode={resolvedTheme}
      className="h-full flex flex-col overflow-hidden"
    >
      <MDEditor
        value={value || ""}
        onChange={(val) => {
          const newValue = val || "";
          if (newValue !== value) {
            onChange(newValue);
          }
        }}
        height={editorHeight}
        preview={showPreview ? "live" : "edit"}
        hideToolbar={false}
        textareaProps={{
          placeholder: placeholder || "输入消息...",
          onKeyDown: (e) => {
            if (
              sendOnEnter && e.key === "Enter" && !e.shiftKey && !e.ctrlKey &&
              !e.metaKey
            ) {
              e.preventDefault();
              const currentValue = (e.target as HTMLTextAreaElement).value;
              if (currentValue.trim()) {
                onChange(currentValue.trim());
                // Trigger send via custom event
                const sendEvent = new CustomEvent("sendMessage", {
                  detail: { content: currentValue.trim() },
                });
                window.dispatchEvent(sendEvent);
              }
            }
          },
        }}
        className="w-full flex-1"
        style={showPreview ? { flex: 1 } : { maxHeight: editorHeight }}
      />
    </div>
  );
}
