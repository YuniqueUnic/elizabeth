"use client";

import { useEffect } from "react";
import { useAppStore } from "@/lib/store";

/**
 * FontSizeManager - 动态管理全局字体大小的组件
 * 该组件监听 Zustand store 中的字体大小配置，并动态更新 CSS 变量
 */
export function FontSizeManager() {
  const editorFontSize = useAppStore((state) => state.editorFontSize);
  const toolbarButtonSize = useAppStore((state) => state.toolbarButtonSize);
  const messageFontSize = useAppStore((state) => state.messageFontSize);

  useEffect(() => {
    // 更新 CSS 变量
    document.documentElement.style.setProperty(
      "--editor-font-size",
      `${editorFontSize}px`,
    );
    document.documentElement.style.setProperty(
      "--toolbar-button-size",
      `${toolbarButtonSize}px`,
    );
    document.documentElement.style.setProperty(
      "--message-font-size",
      `${messageFontSize}px`,
    );
  }, [editorFontSize, toolbarButtonSize, messageFontSize]);

  return null; // 这个组件不渲染任何内容
}
