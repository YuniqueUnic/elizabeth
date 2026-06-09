"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Copy,
  Download,
  ExternalLink,
  FileText,
  Maximize2,
  Minimize2,
  Trash2,
  X,
} from "lucide-react";
import type { FileItem } from "@/lib/types";
import { formatFileSize } from "@/lib/utils/format";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import {
  appendToken,
  buildMarkdownReference,
  buildPreviewMarkdownReference,
  isImageFile,
  resolveFileAssetPath,
  resolveFilePreviewLink,
  toAbsoluteUrl,
} from "@/lib/utils/file-links";
import { downloadFile } from "@/api/fileService";
import { FileContentPreview } from "./file-content-preview";
import dynamic from "next/dynamic";
import { ImageViewer } from "./image-viewer";
import { UrlViewer } from "./url-viewer";
import { getRoomTokenString } from "@/lib/utils/api";
import { insertMarkdownIntoComposer } from "@/lib/composer-editor";
import { useTranslations } from "next-intl";
import { useIsMobile } from "@/hooks/use-mobile";

// Dynamic import to avoid SSR issues with DOMMatrix
const PDFViewer = dynamic(
  () => import("./pdf-viewer").then((mod) => mod.PDFViewer),
  {
    ssr: false,
    loading: () => (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm animate-pulse">
        Loading PDF viewer...
      </div>
    ),
  },
);

interface FilePreviewModalProps {
  file: FileItem | null;
  roomName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDelete: (fileId: string) => void;
}

export function FilePreviewModal(
  { file, roomName, open, onOpenChange, onDelete }: FilePreviewModalProps,
) {
  const t = useTranslations("room");
  const { toast } = useToast();
  const requestInsertMarkdown = useAppStore((state) => state.requestInsertMarkdown);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const { can } = useRoomPermissions();
  const isMobile = useIsMobile();

  if (!file) return null;

  const isImage = isImageFile(file);
  const isLink = file.type === "link";
  const isVideo = file.type === "video" || /\.(mp4|webm|ogg)$/i.test(file.name);
  const isPdf = /\.pdf$/i.test(file.name);
  const isTextFile = /\.(md|markdown|txt|log|json|xml|html|css|js|jsx|ts|tsx|py|java|c|cpp|cs|go|rs|rb|php|swift|kt|scala|sh|bash|yml|yaml|toml|sql|csv|ini|conf|cfg|env)$/i.test(file.name);

  const roomToken = getRoomTokenString(roomName) ?? undefined;
  const assetPath = resolveFileAssetPath(file);
  const authenticatedAssetPath = assetPath
    ? (isLink ? assetPath : appendToken(assetPath, roomToken))
    : undefined;

  // Build a token-free shareable URL for clipboard
  function buildShareableUrl(): string {
    const previewPath = resolveFilePreviewLink(file!);
    if (/^https?:\/\//.test(previewPath)) return previewPath;
    return toAbsoluteUrl(previewPath, window.location.origin);
  }

  const handleDownload = async () => {
    try {
      toast({ title: t("filePreviewModal.downloadStart"), description: t("filePreviewModal.downloading", { fileName: file.name }) });
      await downloadFile(roomName, file.id, file.name);
      toast({ title: t("filePreviewModal.downloadComplete"), description: t("filePreviewModal.downloadCompleteDescription", { fileName: file.name }) });
    } catch {
      toast({ title: t("filePreviewModal.downloadFailed"), description: t("filePreviewModal.downloadFailedDescription"), variant: "destructive" });
    }
  };

  const handleCopyLink = async () => {
    const value = buildShareableUrl();
    try {
      await copyTextToClipboard(value);
      toast({ title: t("filePreviewModal.linkCopied"), description: t("filePreviewModal.linkCopiedDescription") });
    } catch {
      setManualCopyValue(value);
      toast({ title: t("filePreviewModal.copyFailed"), description: t("filePreviewModal.copyFailedDescription"), variant: "destructive" });
    }
  };

  const handleCopyMarkdown = async () => {
    const value = buildMarkdownReference(file!, buildShareableUrl());
    try {
      await copyTextToClipboard(value);
      toast({ title: t("filePreviewModal.markdownCopied"), description: t("filePreviewModal.markdownCopiedDescription") });
    } catch {
      setManualCopyValue(value);
      toast({ title: t("filePreviewModal.copyFailed"), description: t("filePreviewModal.copyFailedDescription"), variant: "destructive" });
    }
  };

  const handleInsertMarkdown = () => {
    const markdown = `\n\n${buildPreviewMarkdownReference(file!)}\n`;
    if (!insertMarkdownIntoComposer(markdown)) requestInsertMarkdown(markdown);
    toast({ title: t("filePreviewModal.insertedToEditor"), description: t("filePreviewModal.insertedToEditorDescription") });
    onOpenChange(false);
  };

  const handleDelete = () => {
    onDelete(file.id);
  };

  // Derive display type label
  const typeLabel = isImage ? "Image"
    : isVideo ? "Video"
    : isPdf ? "PDF"
    : isLink ? "Link"
    : isTextFile ? "Text"
    : "File";

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
      {/*
        Layout: flex column, 3 zones
          1. Header   — file name + meta + window controls (close/fullscreen)
          2. Content  — viewer fills all remaining space, manages its own overflow
          3. Footer   — action bar (download/copy/delete/…)
      */}
      <DialogContent
        className={`
          flex flex-col gap-0 p-0 overflow-hidden transition-all duration-300
          ${isMobile
            ? "max-w-none! w-screen! max-h-[100dvh]! h-[100dvh]! rounded-none! m-0! translate-x-[-50%]! translate-y-[-50%]!"
            : isFullscreen
            ? "max-w-[98vw]! w-[98vw]! max-h-[98vh]! h-[98vh]! rounded-xl"
            : "max-w-4xl w-full h-[88vh] max-h-[88vh] rounded-xl"}
        `}
        // Hide the default shadcn close button; we render our own
        showCloseButton={false}
      >
        {/* ── Zone 1: Header ────────────────────────────────────────── */}
        <div className="flex items-center gap-3 px-4 py-3 border-b bg-muted/20 shrink-0">
          {/* File type badge */}
          <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-muted text-muted-foreground uppercase tracking-wide shrink-0">
            {typeLabel}
          </span>

          {/* File name + size */}
          <div className="flex-1 min-w-0">
            <p className="font-semibold text-sm leading-tight truncate" title={file.name}>
              {file.name}
            </p>
            {file.size
              ? <p className="text-xs text-muted-foreground mt-0.5">{formatFileSize(file.size)}</p>
              : null}
          </div>

          {/* Window controls */}
          <div className="flex items-center gap-1 shrink-0">
            {/* Fullscreen toggle — desktop only (mobile is already full-screen) */}
            {!isMobile && (
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => setIsFullscreen((v) => !v)}
                title={isFullscreen ? t("filePreviewModal.exitFullscreen") : t("filePreviewModal.fullscreen")}
              >
                {isFullscreen
                  ? <Minimize2 className="h-4 w-4" />
                  : <Maximize2 className="h-4 w-4" />}
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => onOpenChange(false)}
              title="Close"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* ── Zone 2: Content ───────────────────────────────────────── */}
        {/*
          Each viewer component is responsible for its own internal layout
          (flex-col h-full, viewer-toolbar + scrollable canvas).
          This outer div is simply a flex-1 container with overflow:hidden.
        */}
        <div className="flex-1 min-h-0 overflow-hidden">
          {isImage && assetPath && (
            <ImageViewer src={assetPath} alt={file.name} roomName={roomName} />
          )}
          {isImage && !assetPath && (
            <EmptyState message={t("filePreviewModal.imageLoadError")} />
          )}

          {isVideo && authenticatedAssetPath && (
            <div className="flex items-center justify-center h-full p-6 bg-black/5">
              <video
                src={authenticatedAssetPath}
                controls
                className="max-w-full max-h-full rounded-lg shadow-lg"
              >
                {t("filePreviewModal.videoNotSupported")}
              </video>
            </div>
          )}

          {isPdf && assetPath && (
            <PDFViewer url={assetPath} roomName={roomName} />
          )}

          {isLink && file.url && (
            <UrlViewer
              url={file.url}
              name={file.name}
              description={file.description ?? file.mimeType}
            />
          )}

          {isTextFile && assetPath && (
            <FileContentPreview
              fileUrl={assetPath}
              fileName={file.name}
              mimeType={file.mimeType}
              roomName={roomName}
            />
          )}

          {!isImage && !isVideo && !isPdf && !isLink && !isTextFile && (
            <EmptyState
              icon={<FileText className="h-10 w-10 opacity-30" />}
              message={t("filePreviewModal.unsupportedType")}
              hint={t("filePreviewModal.downloadToView")}
            />
          )}
        </div>

        {/* ── Zone 3: Footer action bar ─────────────────────────────── */}
        <div className="flex items-center gap-2 px-3 py-2 border-t bg-muted/10 shrink-0 flex-wrap gap-y-1.5">
          {/* Primary actions — left cluster */}
          <Button
            variant="outline"
            size="sm"
            className="h-8"
            onClick={handleDownload}
            data-testid="file-preview-download"
          >
            <Download className="h-3.5 w-3.5 mr-1.5" />
            {t("filePreviewModal.download")}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="h-8"
            onClick={handleCopyLink}
            data-testid="file-preview-copy-link"
          >
            <Copy className="h-3.5 w-3.5 mr-1.5" />
            {t("filePreviewModal.copyLink")}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="h-8"
            onClick={handleCopyMarkdown}
            data-testid="file-preview-copy-markdown"
          >
            <Copy className="h-3.5 w-3.5 mr-1.5" />
            {t("filePreviewModal.copyMarkdown")}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="h-8"
            onClick={handleInsertMarkdown}
            data-testid="file-preview-insert-markdown"
          >
            <ExternalLink className="h-3.5 w-3.5 mr-1.5" />
            {t("filePreviewModal.insertToEditor")}
          </Button>

          {/* Spacer */}
          <div className="flex-1" />

          {/* Destructive action — right edge */}
          <Button
            variant="destructive"
            size="sm"
            className="h-8"
            onClick={handleDelete}
            disabled={!can.delete}
            data-testid="file-preview-delete"
          >
            <Trash2 className="h-3.5 w-3.5 mr-1.5" />
            {t("filePreviewModal.delete")}
          </Button>
        </div>
      </DialogContent>
      </Dialog>
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setManualCopyValue("");
        }}
      />
    </>
  );
}

// ── Shared empty/error state ──────────────────────────────────────────────────
function EmptyState({
  icon,
  message,
  hint,
}: {
  icon?: React.ReactNode;
  message: string;
  hint?: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-3 p-8 text-muted-foreground">
      {icon}
      <p className="text-sm text-center">{message}</p>
      {hint && <p className="text-xs text-center opacity-70">{hint}</p>}
    </div>
  );
}
