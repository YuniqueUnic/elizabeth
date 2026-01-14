"use client";

import {
  BlockTypeSelect,
  BoldItalicUnderlineToggles,
  ButtonWithTooltip,
  CodeToggle,
  CreateLink,
  DiffSourceToggleWrapper,
  InsertCodeBlock,
  InsertImage,
  ListsToggle,
  MDXEditor,
  type MDXEditorMethods,
  codeBlockPlugin,
  codeMirrorPlugin,
  diffSourcePlugin,
  headingsPlugin,
  imagePlugin,
  linkDialogPlugin,
  linkPlugin,
  listsPlugin,
  markdownShortcutPlugin,
  quotePlugin,
  tablePlugin,
  thematicBreakPlugin,
  toolbarPlugin,
  UndoRedo,
} from "@mdxeditor/editor";
import { Paperclip } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { uploadFile } from "@/api/fileService";
import type { FileItem } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { registerComposerEditor, unregisterComposerEditor } from "@/lib/composer-editor";
import "@mdxeditor/editor/style.css";

interface EnhancedMarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  onRequestSend?: () => void;
  placeholder?: string;
  height?: number | string;
  disabled?: boolean;
  sendOnEnter: boolean;
  diffMarkdown?: string;
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

function UploadFileButton({
  disabled,
  onUpload,
}: {
  disabled?: boolean;
  onUpload: (files: File[]) => void | Promise<void>;
}) {
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <>
      <ButtonWithTooltip
        title="上传文件"
        onClick={() => inputRef.current?.click()}
        disabled={disabled}
      >
        <Paperclip />
      </ButtonWithTooltip>
      <input
        ref={inputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => {
          const files = Array.from(e.target.files || []);
          if (files.length > 0) {
            void onUpload(files);
            e.target.value = "";
          }
        }}
      />
    </>
  );
}

export function EnhancedMarkdownEditor({
  value,
  onChange,
  onRequestSend,
  placeholder,
  height = 120,
  disabled,
  sendOnEnter,
  diffMarkdown,
}: EnhancedMarkdownEditorProps) {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const roomName = useAppStore((state) => state.currentRoomId);
  const incrementActiveUploads = useAppStore((state) => state.incrementActiveUploads);
  const decrementActiveUploads = useAppStore((state) => state.decrementActiveUploads);
  const composerInsertRequest = useAppStore((state) => state.composerInsertRequest);
  const clearInsertMarkdownRequest = useAppStore((state) => state.clearInsertMarkdownRequest);

  const wrapperRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<MDXEditorMethods>(null);
  const lastMarkdownRef = useRef(value);
  const lastInsertRequestIdRef = useRef<number | null>(null);

  useEffect(() => {
    const editor = editorRef.current;
    registerComposerEditor(editor);
    return () => unregisterComposerEditor(editor);
  }, []);

  const handleUpload = useCallback(
    async (file: File): Promise<{ markdown: string; url: string }> => {
      if (!roomName) {
        throw new Error("Room name missing");
      }

      incrementActiveUploads();
      try {
        const uploaded = await uploadFile(roomName, file);
        queryClient.invalidateQueries({ queryKey: ["files", roomName] });
        queryClient.invalidateQueries({ queryKey: ["room", roomName] });

        const markdown = buildMarkdownForFile(roomName, uploaded, file);
        const url = fileToRoomPath(roomName, uploaded);

        toast({
          title: "上传成功",
          description: uploaded.name,
        });

        return { markdown, url };
      } finally {
        decrementActiveUploads();
      }
    },
    [
      decrementActiveUploads,
      incrementActiveUploads,
      queryClient,
      roomName,
      toast,
    ],
  );

  const handleUploadFiles = useCallback(
    async (files: File[]) => {
      for (const file of files) {
        const { markdown } = await handleUpload(file);
        editorRef.current?.focus(() => {
          editorRef.current?.insertMarkdown(markdown);
        });
      }
    },
    [handleUpload],
  );

  const imageUploadHandler = useCallback(
    async (image: File): Promise<string> => {
      const { url } = await handleUpload(image);
      return url;
    },
    [handleUpload],
  );

  const plugins = useMemo(() => {
    return [
      headingsPlugin(),
      listsPlugin(),
      quotePlugin(),
      linkPlugin(),
      linkDialogPlugin(),
      tablePlugin(),
      thematicBreakPlugin(),
      markdownShortcutPlugin(),
      codeBlockPlugin({ defaultCodeBlockLanguage: "txt" }),
      codeMirrorPlugin({
        codeBlockLanguages: {
          txt: "Text",
          js: "JavaScript",
          ts: "TypeScript",
          json: "JSON",
          bash: "Bash",
          rust: "Rust",
        },
        autoLoadLanguageSupport: true,
      }),
      imagePlugin({ imageUploadHandler }),
      toolbarPlugin({
        toolbarContents: () => (
          <DiffSourceToggleWrapper
            options={["rich-text", "source", "diff"]}
            SourceToolbar={(
              <>
                <UndoRedo />
                <UploadFileButton disabled={disabled} onUpload={handleUploadFiles} />
              </>
            )}
          >
            <UndoRedo />
            <BoldItalicUnderlineToggles />
            <CodeToggle />
            <ListsToggle />
            <BlockTypeSelect />
            <CreateLink />
            <InsertImage />
            <InsertCodeBlock />
            <UploadFileButton disabled={disabled} onUpload={handleUploadFiles} />
          </DiffSourceToggleWrapper>
        ),
      }),
      diffSourcePlugin({
        viewMode: "rich-text",
        diffMarkdown: diffMarkdown ?? "",
      }),
    ];
  }, [diffMarkdown, disabled, handleUploadFiles, imageUploadHandler]);

  useEffect(() => {
    if (value === lastMarkdownRef.current) return;
    if (!editorRef.current) return;
    editorRef.current.setMarkdown(value);
    lastMarkdownRef.current = value;
  }, [value]);

  useLayoutEffect(() => {
    const request = composerInsertRequest;
    if (!request) return;

    if (lastInsertRequestIdRef.current === request.id) return;
    lastInsertRequestIdRef.current = request.id;

    editorRef.current?.focus(() => {
      editorRef.current?.insertMarkdown(request.markdown);
    });

    clearInsertMarkdownRequest(request.id);
  }, [composerInsertRequest, clearInsertMarkdownRequest]);

  useEffect(() => {
    const root = wrapperRef.current;
    if (!root) return;

    const editable = root.querySelector<HTMLElement>(".mdxeditor-content[contenteditable]");
    if (!editable) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (!onRequestSend || disabled) return;
      if (event.key !== "Enter") return;

      const isSend = sendOnEnter
        ? !event.shiftKey && !event.ctrlKey && !event.metaKey
        : (event.ctrlKey || event.metaKey);

      if (!isSend) return;
      event.preventDefault();
      onRequestSend();
    };

    editable.addEventListener("keydown", handleKeyDown);
    return () => {
      editable.removeEventListener("keydown", handleKeyDown);
    };
  }, [disabled, onRequestSend, sendOnEnter]);

  useEffect(() => {
    const root = wrapperRef.current;
    if (!root) return;

    const editable = root.querySelector<HTMLElement>(".mdxeditor-content[contenteditable]");
    if (!editable) return;

    if (placeholder) {
      editable.dataset.placeholder = placeholder;
    } else {
      delete editable.dataset.placeholder;
    }

    editable.dataset.empty = value.trim() ? "false" : "true";
  }, [placeholder, value]);

  useEffect(() => {
    const root = wrapperRef.current;
    if (!root) return;

    const editable = root.querySelector<HTMLElement>(".mdxeditor-content[contenteditable]");
    if (!editable) return;

    const setFocused = (focused: boolean) => {
      editable.dataset.focused = focused ? "true" : "false";
    };

    setFocused(false);

    const handleFocusIn = () => {
      registerComposerEditor(editorRef.current);
      setFocused(true);
    };
    const handleFocusOut = () => setFocused(false);

    editable.addEventListener("focusin", handleFocusIn);
    editable.addEventListener("focusout", handleFocusOut);

    return () => {
      editable.removeEventListener("focusin", handleFocusIn);
      editable.removeEventListener("focusout", handleFocusOut);
    };
  }, []);

  return (
    <div
      ref={wrapperRef}
      className="h-full flex flex-col min-h-0 overflow-visible"
      data-testid="message-input-editor"
      style={{ height }}
    >
      <MDXEditor
        ref={editorRef}
        markdown={value}
        contentEditableClassName="prose prose-sm dark:prose-invert max-w-none flex-1 min-h-0 overflow-auto mdxeditor-content relative"
        onChange={(markdown) => {
          lastMarkdownRef.current = markdown;
          onChange(markdown);
        }}
        onError={(payload) => {
          console.error("[MDXEditor] markdown parse error:", payload);
        }}
        plugins={plugins}
      />
    </div>
  );
}
