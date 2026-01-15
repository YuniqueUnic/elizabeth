"use client";

import { useEditor, EditorContent, Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import Underline from "@tiptap/extension-underline";
import Link from "@tiptap/extension-link";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { Markdown } from "@tiptap/markdown";
import { common, createLowlight } from "lowlight";
import { useCallback, useEffect, useRef, useImperativeHandle, forwardRef, useState } from "react";
import { useTheme } from "next-themes";
import { useQueryClient } from "@tanstack/react-query";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { uploadFile } from "@/api/fileService";
import type { FileItem } from "@/lib/types";
import { registerComposerEditor, unregisterComposerEditor } from "@/lib/composer-editor";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ImageAuth } from "./tiptap-extensions/image-auth";
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
  Image as ImageIcon,
  Link as LinkIcon,
  Paperclip,
  FileCode,
} from "lucide-react";
import { Textarea } from "@/components/ui/textarea";

const lowlight = createLowlight(common);

// 辅助函数：获取 markdown 内容
function getMarkdownFromEditor(editor: Editor | null): string {
  if (!editor) return "";
  try {
    const doc = editor.getJSON();
    return editor.storage.markdown?.manager?.serialize(doc) || editor.getText();
  } catch (e) {
    console.error("Failed to serialize markdown:", e);
    return editor.getText();
  }
}

// 辅助函数：设置 markdown 内容
function setMarkdownToEditor(editor: Editor, markdown: string): void {
  try {
    const json = editor.storage.markdown?.manager?.parse(markdown);
    if (json) {
      editor.commands.setContent(json);
    } else {
      editor.commands.setContent(markdown);
    }
  } catch (e) {
    console.error("Failed to parse markdown:", e);
    editor.commands.setContent(markdown);
  }
}

// 辅助函数：在 Textarea 中应用 Markdown 语法
function applyMarkdownSyntax(
  textarea: HTMLTextAreaElement,
  format: string,
  value: string,
  onChange: (value: string) => void
) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const selection = value.substring(start, end);
  let before = value.substring(0, start);
  let after = value.substring(end);
  let newSelection = selection;
  let cursorOffset = 0;

  switch (format) {
    case "bold":
      newSelection = `**${selection}**`;
      cursorOffset = 2;
      break;
    case "italic":
      newSelection = `*${selection}*`;
      cursorOffset = 1;
      break;
    case "strike":
      newSelection = `~~${selection}~~`;
      cursorOffset = 2;
      break;
    case "code":
      newSelection = `\`${selection}\``;
      cursorOffset = 1;
      break;
    case "heading-1":
      before = `${before}# `;
      cursorOffset = 2;
      break;
    case "heading-2":
      before = `${before}## `;
      cursorOffset = 3;
      break;
    case "heading-3":
      before = `${before}### `;
      cursorOffset = 4;
      break;
    case "bulletList":
      before = `${before}- `;
      cursorOffset = 2;
      break;
    case "orderedList":
      before = `${before}1. `;
      cursorOffset = 3;
      break;
    case "blockquote":
      before = `${before}> `;
      cursorOffset = 2;
      break;
  }

  const newValue = before + newSelection + after;
  onChange(newValue);

  // 恢复焦点和光标位置
  requestAnimationFrame(() => {
    textarea.focus();
    if (selection) {
      textarea.setSelectionRange(start + cursorOffset, end + cursorOffset);
    } else {
      textarea.setSelectionRange(start + cursorOffset, start + cursorOffset);
    }
  });
}

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
}

function isLikelyImageFile(file: File): boolean {
  if (file.type.startsWith("image/")) return true;
  return /\.(png|jpe?g|gif|webp|svg)$/i.test(file.name);
}

function fileToRoomPath(roomName: string, file: FileItem): string {
  if (file.url) return file.url;
  return `/rooms/${encodeURIComponent(roomName)}/contents/${file.id}`;
}

function buildMarkdownForFile(
  roomName: string,
  file: FileItem,
  original: File,
): string {
  const href = fileToRoomPath(roomName, file);
  if (isLikelyImageFile(original)) {
    return `\n\n![](${href})\n`;
  }
  return `\n\n[${file.name}](${href})\n`;
}

// Toolbar 组件
function EditorToolbar({ editor, onUpload, disabled, isSourceMode, onToggleSourceMode, onAction }: {
  editor: Editor | null;
  onUpload: (files: File[]) => void;
  disabled?: boolean;
  isSourceMode: boolean;
  onToggleSourceMode: () => void;
  onAction: (format: string) => void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (!editor) return null;

  const ToolbarButton = ({
    onClick,
    active,
    children,
    title,
    disabled: buttonDisabled
  }: {
    onClick: () => void;
    active?: boolean;
    children: React.ReactNode;
    title: string;
    disabled?: boolean;
  }) => (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      onClick={onClick}
      disabled={disabled || buttonDisabled}
      className={cn(
        "h-8 w-8 p-0",
        active && "bg-muted"
      )}
      title={title}
    >
      {children}
    </Button>
  );

  const handleAction = (format: string, run: () => void) => {
    if (isSourceMode) {
      onAction(format);
    } else {
      run();
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-0.5 border-b border-border bg-muted/30 p-1">
      <ToolbarButton
        onClick={() => handleAction("bold", () => editor.chain().focus().toggleBold().run())}
        active={!isSourceMode && editor.isActive("bold")}
        title="粗体 (Ctrl+B)"
      >
        <Bold className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("italic", () => editor.chain().focus().toggleItalic().run())}
        active={!isSourceMode && editor.isActive("italic")}
        title="斜体 (Ctrl+I)"
      >
        <Italic className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("strike", () => editor.chain().focus().toggleStrike().run())}
        active={!isSourceMode && editor.isActive("strike")}
        title="删除线"
      >
        <Strikethrough className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("code", () => editor.chain().focus().toggleCode().run())}
        active={!isSourceMode && editor.isActive("code")}
        title="行内代码"
      >
        <Code className="h-4 w-4" />
      </ToolbarButton>

      <div className="w-px h-6 bg-border mx-1" />

      <ToolbarButton
        onClick={() => handleAction("heading-1", () => editor.chain().focus().toggleHeading({ level: 1 }).run())}
        active={!isSourceMode && editor.isActive("heading", { level: 1 })}
        title="标题 1"
      >
        <Heading1 className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("heading-2", () => editor.chain().focus().toggleHeading({ level: 2 }).run())}
        active={!isSourceMode && editor.isActive("heading", { level: 2 })}
        title="标题 2"
      >
        <Heading2 className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("heading-3", () => editor.chain().focus().toggleHeading({ level: 3 }).run())}
        active={!isSourceMode && editor.isActive("heading", { level: 3 })}
        title="标题 3"
      >
        <Heading3 className="h-4 w-4" />
      </ToolbarButton>

      <div className="w-px h-6 bg-border mx-1" />

      <ToolbarButton
        onClick={() => handleAction("bulletList", () => editor.chain().focus().toggleBulletList().run())}
        active={!isSourceMode && editor.isActive("bulletList")}
        title="无序列表"
      >
        <List className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("orderedList", () => editor.chain().focus().toggleOrderedList().run())}
        active={!isSourceMode && editor.isActive("orderedList")}
        title="有序列表"
      >
        <ListOrdered className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("blockquote", () => editor.chain().focus().toggleBlockquote().run())}
        active={!isSourceMode && editor.isActive("blockquote")}
        title="引用"
      >
        <Quote className="h-4 w-4" />
      </ToolbarButton>

      <div className="w-px h-6 bg-border mx-1" />

      <ToolbarButton
        onClick={() => handleAction("undo", () => editor.chain().focus().undo().run())}
        title="撤销 (Ctrl+Z)"
      >
        <Undo className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => handleAction("redo", () => editor.chain().focus().redo().run())}
        title="重做 (Ctrl+Shift+Z)"
      >
        <Redo className="h-4 w-4" />
      </ToolbarButton>

      <div className="w-px h-6 bg-border mx-1" />

      <ToolbarButton
        onClick={() => fileInputRef.current?.click()}
        title="上传文件"
      >
        <Paperclip className="h-4 w-4" />
      </ToolbarButton>

      <div className="flex-1" />

      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onToggleSourceMode}
        disabled={disabled}
        className={cn(
          "h-8 w-8 p-0",
          isSourceMode && "bg-muted"
        )}
        title={isSourceMode ? "切换回预览模式" : "切换到源码模式"}
      >
        <FileCode className="h-4 w-4" />
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

export const MinimalTiptapEditor = forwardRef<MinimalTiptapEditorMethods, MinimalTiptapEditorProps>(
  function MinimalTiptapEditor(
    { value, onChange, onRequestSend, placeholder, disabled, sendOnEnter = true, className },
    ref
  ) {
    const { toast } = useToast();
    const queryClient = useQueryClient();
    const { resolvedTheme } = useTheme();
    const roomName = useAppStore((state) => state.currentRoomId);
    const incrementActiveUploads = useAppStore((state) => state.incrementActiveUploads);
    const decrementActiveUploads = useAppStore((state) => state.decrementActiveUploads);
    const composerInsertRequest = useAppStore((state) => state.composerInsertRequest);
    const clearInsertMarkdownRequest = useAppStore((state) => state.clearInsertMarkdownRequest);
    const editorFontSize = useAppStore((state) => state.editorFontSize);
    const [isSourceMode, setIsSourceMode] = useState(false);
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const lastInsertRequestIdRef = useRef<number | null>(null);
    const isUpdatingFromProp = useRef(false);

    const editor = useEditor({
      extensions: [
        StarterKit.configure({
          codeBlock: false, // 使用 CodeBlockLowlight 替代
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
          placeholder: placeholder || "输入消息...",
        }),
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
          },
        }),
      ],
      content: value,
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
          if (event.key === "Enter" && !event.shiftKey && sendOnEnter && onRequestSend) {
            event.preventDefault();
            onRequestSend();
            return true;
          }
          return false;
        },
      },
      onUpdate: ({ editor }) => {
        if (!isUpdatingFromProp.current) {
          const markdown = getMarkdownFromEditor(editor);
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
        editor?.commands.insertContent(markdown);
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
          editor.commands.insertContent(markdown);
        },
      } : null;

      registerComposerEditor(editorMethods as any);
      return () => unregisterComposerEditor(editorMethods as any);
    }, [editor]);

    // 同步 value prop 到 editor
    useEffect(() => {
      // 如果处于源码模式，不需要同步 Tiptap editor，因为我们直接显示 value
      // 但如果 value 改变了（比如外部更新），我们需要确保 editor 状态也正确，以便切换回来时是对的
      if (editor && value !== getMarkdownFromEditor(editor) && value !== editor.getText()) {
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
        editor.commands.insertContent(request.markdown);
      }

      clearInsertMarkdownRequest(request.id);
    }, [composerInsertRequest, editor, clearInsertMarkdownRequest, isSourceMode, value, onChange]);

    // 处理工具栏操作
    const handleToolbarAction = useCallback((format: string) => {
      if (textareaRef.current) {
        if (format === "undo") {
          textareaRef.current.focus();
          document.execCommand("undo");
          return;
        }
        if (format === "redo") {
          textareaRef.current.focus();
          document.execCommand("redo");
          return;
        }
        applyMarkdownSyntax(textareaRef.current, format, value, onChange);
      }
    }, [value, onChange]);

    // 文件上传处理
    const handleUploadFiles = useCallback(
      async (files: File[]) => {
        if (!roomName || !editor) return;

        for (const file of files) {
          incrementActiveUploads();
          try {
            const uploaded = await uploadFile(roomName, file);
            queryClient.invalidateQueries({ queryKey: ["files", roomName] });
            queryClient.invalidateQueries({ queryKey: ["room", roomName] });

            const markdown = buildMarkdownForFile(roomName, uploaded, file);

            if (isSourceMode) {
              onChange(value + markdown);
            } else {
              editor.commands.insertContent(markdown);
            }

            toast({
              title: "上传成功",
              description: uploaded.name,
            });
          } catch (error: any) {
            toast({
              title: "上传失败",
              description: error.message || "文件上传失败，请重试",
              variant: "destructive",
            });
          } finally {
            decrementActiveUploads();
          }
        }
      },
      [roomName, editor, incrementActiveUploads, decrementActiveUploads, queryClient, toast, isSourceMode, value, onChange]
    );

    return (
      <div
        className={cn(
          "flex flex-col border border-border rounded-md overflow-hidden bg-card tiptap-editor-container",
          disabled && "opacity-50 pointer-events-none",
          className
        )}
        style={{ fontSize: `${editorFontSize}px` }}
      >
        <EditorToolbar
          editor={editor}
          onUpload={handleUploadFiles}
          disabled={disabled}
          isSourceMode={isSourceMode}
          onToggleSourceMode={() => setIsSourceMode(!isSourceMode)}
          onAction={handleToolbarAction}
        />
        {isSourceMode ? (
          <div className="flex-1 min-h-0 relative">
            <Textarea
              ref={textareaRef}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              className="w-full h-full resize-none p-3 font-mono border-0 focus-visible:ring-0 rounded-none bg-transparent"
              placeholder={placeholder}
              disabled={disabled}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey && sendOnEnter && onRequestSend) {
                  e.preventDefault();
                  onRequestSend();
                }
              }}
              style={{ fontSize: `${editorFontSize}px` }}
            />
          </div>
        ) : (
          <EditorContent editor={editor} className="tiptap-editor-content" />
        )}
      </div>
    );
  }
);
