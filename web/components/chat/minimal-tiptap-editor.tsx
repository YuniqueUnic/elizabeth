"use client";

import { useEditor, EditorContent, Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Placeholder from "@tiptap/extension-placeholder";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { Markdown } from "@tiptap/markdown";
import { common, createLowlight } from "lowlight";
import { useCallback, useEffect, useRef, useImperativeHandle, forwardRef, useState, createContext, useContext } from "react";
import { useTranslations } from "next-intl";
import { useTheme } from "next-themes";
import { useQueryClient, useQuery } from "@tanstack/react-query";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { uploadFile } from "@/api/fileService";
import { getRoomDetails } from "@/api/roomService";
import type { FileItem } from "@/lib/types";
import { registerComposerEditor, unregisterComposerEditor } from "@/lib/composer-editor";
import { cn } from "@/lib/utils";
import { generateUUID } from "@/lib/utils/uuid";
import { Button } from "@/components/ui/button";
import { ImageAuth } from "./tiptap-extensions/image-auth";
import { Textarea } from "@/components/ui/textarea";
import {
  getMarkdownFromEditor,
  setMarkdownToEditor,
  insertMarkdownToEditor,
  applyMarkdownSyntax,
  buildMarkdownForFile,
} from "./editor/helpers";
import { EditorToolbar } from "./editor/editor-toolbar";

const lowlight = createLowlight(common);

export interface MinimalTiptapEditorMethods {
  setMarkdown: (markdown: string) => void;
  getMarkdown: () => string;
  focus: (options?: { position?: "start" | "end" | "all" }) => void;
  insertMarkdown: (markdown: string) => void;
}

interface MinimalTiptapEditorProps {
  value: string;
  onChange: (value: string) => void;
  onRequestSend?: () => void;
  placeholder?: string;
  disabled?: boolean;
  sendOnEnter?: boolean;
  className?: string;
  toolbarPosition?: "top" | "bottom" | "none";
  editorClassName?: string;
  renderActions?: () => React.ReactNode;
}

export const MinimalTiptapEditor = forwardRef<MinimalTiptapEditorMethods, MinimalTiptapEditorProps>(
  function MinimalTiptapEditor(
    {
      value,
      onChange,
      onRequestSend,
      placeholder,
      disabled,
      sendOnEnter = true,
      className,
      toolbarPosition = "top",
      editorClassName,
      renderActions
    },
    ref
  ) {
    const { toast } = useToast();
    const t = useTranslations("room.messageInput");
    const queryClient = useQueryClient();
    const { resolvedTheme } = useTheme();
    const roomName = useAppStore((state) => state.currentRoomId);
    const { data: roomDetails } = useQuery({
      queryKey: ["room", roomName],
      queryFn: () => getRoomDetails(roomName),
      staleTime: 1000,
      enabled: !!roomName,
    });
    const addTransfer = useAppStore((state) => state.addTransfer);
    const updateTransferStatus = useAppStore((state) => state.updateTransferStatus);
    const removeTransfer = useAppStore((state) => state.removeTransfer);
    const composerInsertRequest = useAppStore((state) => state.composerInsertRequest);
    const clearInsertMarkdownRequest = useAppStore((state) => state.clearInsertMarkdownRequest);
    const editorFontSize = useAppStore((state) => state.editorFontSize);
    const [isSourceMode, setIsSourceMode] = useState(false);
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    // 用 ref 缓存最新的 sendOnEnter/onRequestSend，避免 useEditor 闭包捕获陈旧值
    const sendOnEnterRef = useRef(sendOnEnter);
    const onRequestSendRef = useRef(onRequestSend);
    useEffect(() => { sendOnEnterRef.current = sendOnEnter; }, [sendOnEnter]);
    useEffect(() => { onRequestSendRef.current = onRequestSend; }, [onRequestSend]);

    const lastInsertRequestIdRef = useRef<number | null>(null);
    const isUpdatingFromProp = useRef(false);
    const locallyEmittedValuesRef = useRef(new Set<string>());

    const editor = useEditor({
      extensions: [
        StarterKit.configure({
          codeBlock: false, // 使用 CodeBlockLowlight 替代
          link: false,      // 显式关闭内置 Link，使用下面独立配置的版本
        }),
        // 独立配置 Link，以支持 /contents/... 相对路径内部链接
        Link.configure({
          openOnClick: false,     // 编辑器内不直接跳转，由外部处理
          autolink: true,
          HTMLAttributes: {
            class: "text-primary underline underline-offset-2 cursor-pointer",
            rel: "noopener noreferrer",
          },
          validate: (href) =>
            // 允许标准 http/https 链接和 /contents/... 内部相对路径
            /^https?:\/\//.test(href) || href.startsWith("/"),
        }),
        CodeBlockLowlight.configure({
          lowlight,
          defaultLanguage: "plaintext",
        }),
        Markdown.configure({
          markedOptions: {
            gfm: true, // GitHub Flavored Markdown
            breaks: true, // 支持换行
          },
        }),
        Placeholder.configure({
          placeholder: placeholder || t("placeholderDefault"),
        }),
        ImageAuth.configure({
          HTMLAttributes: {
            class: "max-w-full rounded-md border border-border",
          },
        }),
      ],
      content: value,
      contentType: "markdown",
      editorProps: {
        attributes: {
          class: cn(
            "prose prose-sm dark:prose-invert max-w-none",
            "w-full px-3 py-2",
            "focus:outline-none"
          ),
          style: `font-size: ${editorFontSize}px`, // 在编辑器内部也应用字体大小
        },
        handleKeyDown: (view, event) => {
          if (event.key === "Enter") {
            const send = onRequestSendRef.current;
            const soe = sendOnEnterRef.current;
            if (event.ctrlKey || event.metaKey) {
              // Ctrl/Cmd+Enter：始终发送
              if (send) {
                event.preventDefault();
                send();
                return true;
              }
            } else if (!event.shiftKey && soe) {
              // Enter（非 Shift）且 sendOnEnter=true：发送
              if (send) {
                event.preventDefault();
                send();
                return true;
              }
            }
            // Shift+Enter 或 sendOnEnter=false 时的 Enter：换行（默认行为）
          }
          return false;
        },
      },
      onUpdate: ({ editor }) => {
        if (!isUpdatingFromProp.current) {
          const markdown = getMarkdownFromEditor(editor);
          locallyEmittedValuesRef.current.add(markdown);
          onChange(markdown);
        }
      },
      immediatelyRender: false, // 关键：防止 SSR hydration mismatch
    });

    // 暴露方法给父组件
    useImperativeHandle(ref, () => ({
      setMarkdown: (markdown: string) => {
        if (editor && markdown !== getMarkdownFromEditor(editor)) {
          isUpdatingFromProp.current = true;
          setMarkdownToEditor(editor, markdown);
          isUpdatingFromProp.current = false;
        }
      },
      getMarkdown: () => {
        return getMarkdownFromEditor(editor);
      },
      focus: (options) => {
        if (options?.position === "end") {
          editor?.commands.focus("end");
        } else if (options?.position === "start") {
          editor?.commands.focus("start");
        } else {
          editor?.commands.focus();
        }
      },
      insertMarkdown: (markdown: string) => {
        if (editor) {
          insertMarkdownToEditor(editor, markdown);
        }
      },
    }), [editor]);

    // 注册编辑器到全局
    useEffect(() => {
      const editorMethods: MinimalTiptapEditorMethods | null = editor ? {
        setMarkdown: (markdown: string) => {
          if (markdown !== getMarkdownFromEditor(editor)) {
            isUpdatingFromProp.current = true;
            setMarkdownToEditor(editor, markdown);
            isUpdatingFromProp.current = false;
          }
        },
        getMarkdown: () => getMarkdownFromEditor(editor),
        focus: (options) => {
          if (options?.position === "end") {
            editor.commands.focus("end");
          } else if (options?.position === "start") {
            editor.commands.focus("start");
          } else {
            editor.commands.focus();
          }
        },
        insertMarkdown: (markdown: string) => {
          insertMarkdownToEditor(editor, markdown);
        },
      } : null;

      registerComposerEditor(editorMethods as any);
      return () => unregisterComposerEditor(editorMethods as any);
    }, [editor]);

    // 同步 value prop 到 editor
    useEffect(() => {
      // 如果处于源码模式，不需要同步 Tiptap editor，因为我们直接显示 value
      // 但如果 value 改变了（比如外部更新），我们需要确保 editor 状态也正确，以便切换回来时是对的
      if (!editor) return;

      const wasEmittedByThisEditor = locallyEmittedValuesRef.current.delete(value);
      if (
        value !== getMarkdownFromEditor(editor) &&
        value !== editor.getText() &&
        !wasEmittedByThisEditor
      ) {
        isUpdatingFromProp.current = true;
        setMarkdownToEditor(editor, value);
        isUpdatingFromProp.current = false;
      }
    }, [value, editor]);

    // 处理插入请求
    useEffect(() => {
      const request = composerInsertRequest;
      if (!request || !editor) return;

      if (lastInsertRequestIdRef.current === request.id) return;
      lastInsertRequestIdRef.current = request.id;

      if (isSourceMode) {
        // 如果在源码模式，直接追加到 value
        // 这里其实应该通过 update value 来实现，但我们通过 onChange 通知父组件
        const newValue = value + request.markdown;
        onChange(newValue);
      } else {
        editor.commands.focus("end");
        insertMarkdownToEditor(editor, request.markdown);
      }

      clearInsertMarkdownRequest(request.id);
    }, [composerInsertRequest, editor, clearInsertMarkdownRequest, isSourceMode, value, onChange]);

    // 处理工具栏操作（source 模式下的格式化）
    const handleToolbarAction = useCallback((format: string) => {
      if (textareaRef.current) {
        // undo/redo: 浏览器原生 Ctrl+Z / Ctrl+Y 对 textarea 有效，无需手动触发
        if (format === "undo" || format === "redo") return;
        applyMarkdownSyntax(textareaRef.current, format, value, onChange);
      }
    }, [value, onChange]);

    // 文件上传处理
    const handleUploadFiles = useCallback(
      async (files: File[]) => {
        if (!roomName || !editor) return;

        if (roomDetails) {
          const transfers = useAppStore.getState().transfers;
          const activeUploadsSize = Object.values(transfers)
            .filter((t) => t.status === "active" && t.direction === "upload")
            .reduce((sum, t) => sum + (t.fileSize || 0), 0);

          const totalBatchSize = files.reduce((sum, f) => sum + f.size, 0);

          if (roomDetails.currentSize + activeUploadsSize + totalBatchSize > roomDetails.maxSize) {
            toast({
              title: t("uploadFailed"),
              description: t("uploadFailedSizeExceeded"),
              variant: "destructive",
            });
            return;
          }
        }

        for (const file of files) {
          const transferId = generateUUID();
          addTransfer({
            id: transferId,
            fileName: file.name,
            fileSize: file.size,
            direction: "upload",
            status: "active",
            progress: { bytesTransferred: 0, totalBytes: file.size, percentage: 0, speed: 0, estimatedTimeRemaining: 0 },
            startedAt: Date.now(),
            abortController: new AbortController(),
          });
          try {
            const uploaded = await uploadFile(roomName, file);
            updateTransferStatus(transferId, "completed");
            setTimeout(() => removeTransfer(transferId), 3000);
            queryClient.invalidateQueries({ queryKey: ["files", roomName] });
            queryClient.invalidateQueries({ queryKey: ["room", roomName] });

            const markdown = buildMarkdownForFile(roomName, uploaded, file);

            if (isSourceMode) {
              onChange(value + markdown);
            } else {
              insertMarkdownToEditor(editor, markdown);
            }

            toast({
              title: t("uploadSuccess"),
              description: uploaded.name,
            });
          } catch (error: any) {
            updateTransferStatus(transferId, "error", error.message);
            setTimeout(() => removeTransfer(transferId), 5000);

            const isSizeError =
              error?.code === 413 ||
              (error?.code === 403 &&
                (error?.message?.includes("空间不足") ||
                  error?.message?.includes("limit exceeded") ||
                  error?.message?.includes("容量")));

            toast({
              title: t("uploadFailed"),
              description: isSizeError ? t("uploadFailedSizeExceeded") : (error.message || t("uploadFailedDescription")),
              variant: "destructive",
            });
          }
        }
      },
      [roomName, editor, addTransfer, updateTransferStatus, removeTransfer, queryClient, toast, isSourceMode, value, onChange, roomDetails, t]
    );

    return (
      <div
        className={cn(
          "flex flex-col border border-border rounded-xl bg-card transition-all duration-200 focus-within:border-primary/40 focus-within:ring-2 focus-within:ring-primary/10 overflow-hidden tiptap-editor-container",
          disabled && "opacity-50 pointer-events-none",
          className
        )}
        style={{ fontSize: `${editorFontSize}px` }}
      >
        {toolbarPosition === "top" && (
          <EditorToolbar
            editor={editor}
            onUpload={handleUploadFiles}
            disabled={disabled}
            isSourceMode={isSourceMode}
            onToggleSourceMode={() => setIsSourceMode(!isSourceMode)}
            onAction={handleToolbarAction}
            className="border-b border-border bg-muted/20"
          />
        )}

        <div className={cn("flex-1 min-h-0 relative flex flex-col overflow-hidden", editorClassName)}>
          {isSourceMode ? (
            <Textarea
              ref={textareaRef}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              className="w-full h-full resize-none p-4 font-mono border-0 focus-visible:ring-0 rounded-none bg-transparent"
              placeholder={placeholder}
              disabled={disabled}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  if (e.ctrlKey || e.metaKey) {
                    // Ctrl/Cmd+Enter：始终发送
                    if (onRequestSend) {
                      e.preventDefault();
                      onRequestSend();
                    }
                  } else if (!e.shiftKey && sendOnEnter) {
                    // Enter（非 Shift）且 sendOnEnter=true：发送
                    if (onRequestSend) {
                      e.preventDefault();
                      onRequestSend();
                    }
                  }
                  // Shift+Enter 或 sendOnEnter=false 时：换行（默认）
                }
              }}
              style={{ fontSize: `${editorFontSize}px` }}
            />
          ) : (
            <EditorContent editor={editor} className="tiptap-editor-content" />
          )}
        </div>

        {toolbarPosition === "bottom" && (
          <div className="flex items-center justify-between border-t border-border bg-muted/10 p-1.5 min-h-10">
            <EditorToolbar
              editor={editor}
              onUpload={handleUploadFiles}
              disabled={disabled}
              isSourceMode={isSourceMode}
              onToggleSourceMode={() => setIsSourceMode(!isSourceMode)}
              onAction={handleToolbarAction}
              className="flex-1"
            />
            {renderActions && (
              <div className="flex items-center gap-1.5 px-2 shrink-0">
                {renderActions()}
              </div>
            )}
          </div>
        )}
      </div>
    );
  }
);
