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
import { downloadFile } from "@/api/fileService";
import { FileContentPreview } from "./file-content-preview";
import dynamic from "next/dynamic";
import { ImageViewer } from "./image-viewer";
import { UrlViewer } from "./url-viewer";
import { API_BASE_URL } from "@/lib/config";
import { getRoomTokenString } from "@/lib/utils/api";

// Dynamic import for PDFViewer to avoid SSR issues with DOMMatrix
const PDFViewer = dynamic(
  () => import("./pdf-viewer").then((mod) => mod.PDFViewer),
  {
    ssr: false,
    loading: () => (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">加载 PDF 查看器...</p>
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
  const { toast } = useToast();
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const [isFullscreen, setIsFullscreen] = useState(false);

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
        title: "开始下载",
        description: `正在下载 ${file.name}`,
      });
      await downloadFile(currentRoomId, file.id, file.name);
      toast({
        title: "下载完成",
        description: `${file.name} 已成功下载`,
      });
    } catch (error) {
      console.error("Download failed:", error);
      toast({
        title: "下载失败",
        description: "无法下载文件，请重试",
        variant: "destructive",
      });
    }
  };

  const handleCopyLink = () => {
    // ✅ FIX: Copy the actual download URL with full domain
    const downloadUrl = file.url
      ? `${window.location.origin}${file.url}`
      : `${window.location.origin}/api/v1/rooms/${currentRoomId}/contents/${file.id}`;

    navigator.clipboard.writeText(downloadUrl);
    toast({
      title: "链接已复制",
      description: "文件下载链接已复制到剪贴板",
    });
  };

  const handleDelete = () => {
    onDelete(file.id);
    onOpenChange(false);
    toast({
      title: "文件已删除",
      description: `${file.name} 已从房间中删除`,
    });
  };

  const handleOpenInNewTab = () => {
    if (file.url) {
      window.open(file.url, "_blank");
      toast({
        title: "已在新标签页打开",
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
                >
                  <Trash2 className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">删除</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDownload}
                  className="h-8"
                >
                  <Download className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">下载</span>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCopyLink}
                  className="h-8"
                >
                  <Copy className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">复制链接</span>
                </Button>
                <div className="flex-1" />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsFullscreen(!isFullscreen)}
                  title={isFullscreen ? "退出全屏" : "全屏查看"}
                  className="h-8"
                >
                  <Maximize2 className="h-4 w-4 mr-1" />
                  <span className="hidden sm:inline">
                    {isFullscreen ? "退出全屏" : "全屏"}
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
              <p>无法生成图片 URL（缺少 token 或 URL）</p>
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
                您的浏览器不支持视频播放
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
              fileUrl={file.url}
              fileName={file.name}
              mimeType={file.mimeType}
              roomName={currentRoomId}
              onFullscreenToggle={setIsFullscreen}
            />
          )}

          {!isImage && !isVideo && !isPdf && !isLink && !isTextFile && (
            <div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
              <p>无法预览此文件类型</p>
              <p className="text-sm mt-2">请下载文件以查看内容</p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
