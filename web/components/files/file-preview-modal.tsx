"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Copy,
  Download,
  ExternalLink,
  Maximize2,
  Trash2,
  X,
} from "lucide-react";
import type { FileItem } from "@/lib/types";
import { formatFileSize } from "@/lib/utils/format";
import { useToast } from "@/hooks/use-toast";
import { useAppStore } from "@/lib/store";
import { downloadFile } from "@/api/fileService";
import { FileContentPreview } from "./file-content-preview";

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
  const [showIframe, setShowIframe] = useState(false);

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
    navigator.clipboard.writeText(
      file.url || `https://elizabeth.app/files/${file.id}`,
    );
    toast({
      title: "链接已复制",
      description: "文件链接已复制到剪贴板",
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

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <div className="flex-1 truncate pr-4">
              <div className="file-name" title={file.name}>
                {file.name}
              </div>
              <div className="text-sm font-normal text-muted-foreground">
                {file.size ? formatFileSize(file.size) : ""}
              </div>
            </div>
          </DialogTitle>
        </DialogHeader>

        {/* Toolbar */}
        <div className="flex items-center gap-2 border-b pb-3">
          <Button variant="outline" size="sm" onClick={handleDownload}>
            <Download className="h-4 w-4 mr-2" />
            下载
          </Button>
          <Button variant="outline" size="sm" onClick={handleCopyLink}>
            <Copy className="h-4 w-4 mr-2" />
            复制链接
          </Button>
          {isLink && (
            <Button variant="outline" size="sm" onClick={handleOpenInNewTab}>
              <ExternalLink className="h-4 w-4 mr-2" />
              新标签页打开
            </Button>
          )}
          <div className="flex-1" />
          <Button variant="outline" size="sm" onClick={handleDelete}>
            <Trash2 className="h-4 w-4 mr-2" />
            删除
          </Button>
        </div>

        {/* Preview Content */}
        <div className="flex-1 overflow-auto">
          {isImage && (
            <div className="flex items-center justify-center p-4">
              <img
                src={file.thumbnailUrl || file.url || "/placeholder.svg"}
                alt={file.name}
                className="max-w-full max-h-[60vh] object-contain rounded-lg"
              />
            </div>
          )}

          {isVideo && (
            <div className="flex items-center justify-center p-4">
              <video
                src={file.url}
                controls
                className="max-w-full max-h-[60vh] rounded-lg"
              >
                您的浏览器不支持视频播放
              </video>
            </div>
          )}

          {isPdf && (
            <div className="h-[60vh]">
              <iframe
                src={file.url}
                className="w-full h-full border-0 rounded-lg"
                title={file.name}
              />
            </div>
          )}

          {isLink && !showIframe && (
            <div className="flex flex-col items-center justify-center p-8 space-y-4">
              <ExternalLink className="h-16 w-16 text-muted-foreground" />
              <div className="text-center">
                <p className="text-lg font-medium mb-2">外部链接</p>
                <p className="text-sm text-muted-foreground mb-4 break-all max-w-md">
                  {file.url}
                </p>
                <div className="flex gap-2">
                  <Button onClick={() => setShowIframe(true)}>
                    <Maximize2 className="h-4 w-4 mr-2" />
                    在此处加载
                  </Button>
                  <Button variant="outline" onClick={handleOpenInNewTab}>
                    <ExternalLink className="h-4 w-4 mr-2" />
                    新标签页打开
                  </Button>
                </div>
              </div>
            </div>
          )}

          {isLink && showIframe && (
            <div className="relative h-[60vh]">
              <Button
                variant="outline"
                size="sm"
                className="absolute top-2 right-2 z-10 bg-transparent"
                onClick={() => setShowIframe(false)}
              >
                <X className="h-4 w-4 mr-2" />
                关闭预览
              </Button>
              <iframe
                src={file.url}
                className="w-full h-full border-0 rounded-lg"
                title={file.name}
              />
            </div>
          )}

          {/* Text file preview (Markdown, code, plain text) */}
          {isTextFile && file.url && (
            <FileContentPreview
              fileUrl={file.url}
              fileName={file.name}
              mimeType={file.mimeType}
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
