"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Copy, Download, ExternalLink, Maximize2, Trash2 } from "lucide-react";
import type { FileItem } from "@/lib/types";
import { formatFileSize } from "@/lib/utils/format";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { useRoomPermissions } from "@/hooks/use-room-permissions";

import { downloadFile } from "@/api/fileService";
import { FileContentPreview } from "./file-content-preview";
import dynamic from "next/dynamic";
import { ImageViewer } from "./image-viewer";
import { UrlViewer } from "./url-viewer";
import { API_BASE_URL } from "@/lib/config";
import { getRoomTokenString } from "@/lib/utils/api";
import { insertMarkdownIntoComposer } from "@/lib/composer-editor";
import { useTranslations } from "next-intl";

// Dynamic import for PDFViewer to avoid SSR issues with DOMMatrix
const PDFViewer = dynamic(
  () => import("./pdf-viewer").then((mod) => mod.PDFViewer),
  {
    ssr: false,
    loading: () => (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">Loading PDF viewer...</p>
      </div>
    ),
  },
);

interface FilePreviewModalProps {
  file: FileItem | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDelete: (fileId: string) => void;
}

export function FilePreviewModal(
  { file, open, onOpenChange, onDelete }: FilePreviewModalProps,
) {
  const t = useTranslations("room");
  const { toast } = useToast();
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const requestInsertMarkdown = useAppStore((state) => state.requestInsertMarkdown);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const { can } = useRoomPermissions();


  if (!file) return null;

  const isImage = file.type === "image" ||
    file.name.match(/\.(jpg|jpeg|png|gif|webp|svg)$/i);
  const isLink = file.type === "link";
  const isVideo = file.type === "video" ||
    file.name.match(/\.(mp4|webm|ogg)$/i);
  const isPdf = file.name.match(/\.pdf$/i);

  // Check if file is a text-based file that can be previewed
  const isTextFile = file.name.match(
    /\.(md|markdown|txt|log|json|xml|html|css|js|jsx|ts|tsx|py|java|c|cpp|cs|go|rs|rb|php|swift|kt|scala|sh|bash|yml|yaml|toml|sql|csv|ini|conf|cfg|env)$/i,
  );

  const handleDownload = async () => {
    try {
      toast({
        title: t("filePreviewModal.downloadStart"),
        description: t("filePreviewModal.downloading", { fileName: file.name }),
      });
      await downloadFile(currentRoomId, file.id, file.name);
      toast({
        title: t("filePreviewModal.downloadComplete"),
        description: t("filePreviewModal.downloadCompleteDescription", { fileName: file.name }),
      });
    } catch (error) {
      console.error("Download failed:", error);
      toast({
        title: t("filePreviewModal.downloadFailed"),
        description: t("filePreviewModal.downloadFailedDescription"),
        variant: "destructive",
      });
    }
  };

  const handleCopyLink = () => {
    // ✅ FIX: Copy the actual download URL with full domain
    const downloadUrl = file.url
      ? `${window.location.origin}${file.url}`
      : `${window.location.origin}/api/v1/contents/${file.id}`;

    navigator.clipboard.writeText(downloadUrl);
    toast({
      title: t("filePreviewModal.linkCopied"),
      description: t("filePreviewModal.linkCopiedDescription"),
    });
  };

  const buildMarkdownLink = () => {
    const href = file.url ?? `/contents/${file.id}`;
    if (isImage) return `![](${href})`;
    return `[${file.name}](${href})`;
  };

  const handleCopyMarkdown = () => {
    const markdown = buildMarkdownLink();
    navigator.clipboard.writeText(markdown);
    toast({
      title: t("filePreviewModal.markdownCopied"),
      description: t("filePreviewModal.markdownCopiedDescription"),
    });
  };

  const handleInsertMarkdown = () => {
    const markdown = `\n\n${buildMarkdownLink()}\n`;
    if (!insertMarkdownIntoComposer(markdown)) {
      requestInsertMarkdown(markdown);
    }
    toast({
      title: t("filePreviewModal.insertedToEditor"),
      description: t("filePreviewModal.insertedToEditorDescription"),
    });
    onOpenChange(false);
  };

  const handleDelete = () => {
    onDelete(file.id);
    onOpenChange(false);
    toast({
      title: t("filePreviewModal.fileDeleted"),
      description: t("filePreviewModal.fileDeletedDescription", { fileName: file.name }),
    });
  };

  const handleOpenInNewTab = () => {
    if (file.url) {
      window.open(file.url, "_blank");
      toast({
        title: t("filePreviewModal.openedInNewTab"),
      });
    }
  };

  // Get authenticated image URL
  const getAuthenticatedUrl = (url?: string) => {
    if (!url) return undefined;

    // If it's a relative URL, it needs authentication and full URL
    if (url.startsWith("/")) {
      // ✅ FIX: Use getRoomTokenString to get token from unified storage
      const token = getRoomTokenString(currentRoomId);

      if (token) {
        // Build full URL: http://localhost:4092/api/v1 + /rooms/... + ?token=...
        const fullUrl = `${API_BASE_URL}${url}?token=${token}`;
        console.log("Generated authenticated URL:", fullUrl);
        return fullUrl;
      } else {
        console.warn("No token found for room:", currentRoomId);
        return undefined;
      }
    }
    return url;
  };

  const imageUrl = getAuthenticatedUrl(file.url);
  const videoUrl = getAuthenticatedUrl(file.url);
  const pdfUrl = getAuthenticatedUrl(file.url);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className={`${
          isFullscreen
            ? "max-w-[98vw]! w-[98vw]! max-h-[98vh]! h-[98vh]!"
            : "max-w-4xl max-h-[90vh]"
        } flex flex-col transition-all duration-300`}
      >
        <DialogHeader className="space-y-2 pb-2">
          <DialogTitle asChild>
            <div className="space-y-2">
              {/* File Info Row - Compact */}
              <div className="flex items-center gap-2 min-w-0">
                <div className="truncate font-semibold" title={file.name}>
                  {file.name}
                </div>
                <div className="text-sm font-normal text-muted-foreground whitespace-nowrap">
                  {file.size ? formatFileSize(file.size) : ""}
                </div>
              </div>

              {/* Toolbar Row */}
              <div className="flex items-center gap-2 flex-wrap">
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={handleDelete}
                  className="h-8"
                  disabled={!can.delete}
                >
                  <Trash2 className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">{t("filePreviewModal.delete")}</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDownload}
                  className="h-8"
                >
                  <Download className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">{t("filePreviewModal.download")}</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCopyLink}
                  className="h-8"
                >
                  <Copy className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">{t("filePreviewModal.copyLink")}</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCopyMarkdown}
                  className="h-8"
                >
                  <Copy className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">{t("filePreviewModal.copyMarkdown")}</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleInsertMarkdown}
                  className="h-8"
                >
                  <Copy className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">{t("filePreviewModal.insertToEditor")}</span>
                </Button>
                <div className="flex-1" />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsFullscreen(!isFullscreen)}
                  title={isFullscreen ? t("filePreviewModal.exitFullscreen") : t("filePreviewModal.fullscreen")}
                  className="h-8"
                >
                  <Maximize2 className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">
                    {isFullscreen ? t("filePreviewModal.exitFullscreen") : t("filePreviewModal.fullscreen")}
                  </span>
                </Button>
              </div>
            </div>
          </DialogTitle>
        </DialogHeader>

        {/* Preview Content */}
        <div className="flex-1 overflow-auto">
          {/* Image Preview with Enhanced Viewer */}
          {isImage && imageUrl && (
            <ImageViewer
              src={imageUrl}
              alt={file.name}
              className="max-w-full max-h-full object-contain"
            />
          )}
          {isImage && !imageUrl && (
            <div className="flex items-center justify-center h-full p-4 text-muted-foreground">
              <p>{t("filePreviewModal.imageLoadError")}</p>
            </div>
          )}

          {/* Video Preview */}
          {isVideo && videoUrl && (
            <div className="flex items-center justify-center h-full p-4">
              <video
                src={videoUrl}
                controls
                className="max-w-full max-h-full rounded-lg"
              >
                {t("filePreviewModal.videoNotSupported")}
              </video>
            </div>
          )}

          {/* PDF Preview with Enhanced Viewer */}
          {isPdf && pdfUrl && <PDFViewer url={pdfUrl} />}

          {/* URL/Link Preview with Enhanced Viewer */}
          {isLink && file.url && (
            <UrlViewer
              url={file.url}
              name={file.name}
              description={file.mimeType}
            />
          )}

          {/* Text file preview (Markdown, code, plain text) */}
          {isTextFile && file.url && (
            <FileContentPreview
              fileUrl={`/rooms/${currentRoomId}/contents/${file.id}`}
              fileName={file.name}
              mimeType={file.mimeType}
              roomName={currentRoomId}
              onFullscreenToggle={setIsFullscreen}
            />
          )}

          {!isImage && !isVideo && !isPdf && !isLink && !isTextFile && (
            <div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
              <p>{t("filePreviewModal.unsupportedType")}</p>
              <p className="text-sm mt-2">{t("filePreviewModal.downloadToView")}</p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
