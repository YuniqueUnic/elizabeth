"use client";

import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { FileListView } from "@/components/files/file-list-view";
import { FileUploadZone } from "@/components/files/file-upload-zone";
import { FilePreviewModal } from "@/components/files/file-preview-modal";
import {
  type UrlUploadData,
  UrlUploadDialog,
} from "@/components/files/url-upload-dialog";
import {
  CheckSquare,
  Download,
  Link as LinkIcon,
  Repeat,
  Square,
  Upload,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteFile,
  downloadFilesBatch,
  getFilesList,
  uploadFile,
  uploadUrl,
} from "@/api/fileService";
import { useAppStore } from "@/lib/store";
import { useToast } from "@/hooks/use-toast";
import { useIsMobile } from "@/hooks/use-mobile";
import { useRoomPermissions } from "@/hooks/use-room-permissions";
import type { FileItem } from "@/lib/types";
import {
  handleMutationError,
  handleMutationSuccess,
} from "@/lib/utils/mutations";
import { useParams } from "next/navigation";

export function RightSidebar() {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const selectedFiles = useAppStore((state) => state.selectedFiles);
  const clearFileSelection = useAppStore((state) => state.clearFileSelection);
  const selectAllFiles = useAppStore((state) => state.selectAllFiles);
  const invertFileSelection = useAppStore((state) => state.invertFileSelection);
  const incrementActiveUploads = useAppStore((state) => state.incrementActiveUploads);
  const decrementActiveUploads = useAppStore((state) => state.decrementActiveUploads);
  const activeUploads = useAppStore((state) => state.activeUploads);
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const isMobile = useIsMobile();
  const { can } = useRoomPermissions();
  const params = useParams();
  const roomName = (params.roomName as string) || currentRoomId;

  const [previewFile, setPreviewFile] = useState<FileItem | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [urlDialogOpen, setUrlDialogOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const { data: files = [], isLoading } = useQuery({
    queryKey: ["files", roomName],
    queryFn: () => getFilesList(roomName),
    staleTime: 4000,
    enabled: !!roomName,
  });

  const uploadMutation = useMutation({
    mutationFn: (file: File) => uploadFile(roomName, file),
    onMutate: () => incrementActiveUploads(),
    onSettled: () => decrementActiveUploads(),
    onSuccess: () => {
      console.log(
        `[uploadMutation.onSuccess] 上传成功，失效缓存 roomName=${roomName}`,
      );
      queryClient.invalidateQueries({ queryKey: ["files", roomName] });
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] });
      handleMutationSuccess(toast, {
        title: "上传成功",
        description: "文件已成功上传到房间",
      });
    },
    onError: (error) => {
      console.error(`[uploadMutation.onError] 上传失败:`, error);
      handleMutationError(error, toast, {
        description: "文件上传失败，请重试",
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (fileId: string) => deleteFile(roomName, fileId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["files", roomName] });
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] });
      handleMutationSuccess(toast, {
        title: "删除成功",
        description: "文件已从房间中删除",
      });
    },
    onError: (error) => {
      handleMutationError(error, toast, {
        description: "无法删除文件，请重试",
      });
    },
  });

  const uploadUrlMutation = useMutation({
    mutationFn: (data: UrlUploadData) => uploadUrl(roomName, data),
    onMutate: () => incrementActiveUploads(),
    onSettled: () => decrementActiveUploads(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["files", roomName] });
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] });
      handleMutationSuccess(toast, {
        title: "链接已添加",
        description: "URL 已成功添加到房间",
      });
      setUrlDialogOpen(false);
    },
    onError: (error) => {
      handleMutationError(error, toast, {
        description: "添加链接失败，请重试",
      });
    },
  });

  const handleUpload = async (acceptedFiles: File[]) => {
    for (const file of acceptedFiles) {
      await uploadMutation.mutateAsync(file);
    }
  };

  const handleDelete = (fileId: string) => {
    deleteMutation.mutate(fileId);
  };

  const handleBatchDownload = async () => {
    if (selectedFiles.size > 0) {
      toast({
        title: "开始下载",
        description: `正在准备下载 ${selectedFiles.size} 个文件`,
      });
      await downloadFilesBatch(roomName, Array.from(selectedFiles));
      clearFileSelection();
    }
  };

  const handleFileClick = (file: FileItem) => {
    setPreviewFile(file);
    setPreviewOpen(true);
  };

  const selectedCount = selectedFiles.size;
  const fileIds = files.map((f) => f.id);
  const allSelected = selectedFiles.size === files.length && files.length > 0;

  return (
    <>
      <aside
        className={`flex flex-col bg-muted/30 h-full overflow-hidden ${
          isMobile ? "w-full" : "w-80 border-l"
        }`}
      >
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">文件管理</h2>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="icon"
              title="添加链接"
              disabled={uploadUrlMutation.isPending || !can.edit}
              onClick={() => setUrlDialogOpen(true)}
            >
              <LinkIcon className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              title="上传文件"
              disabled={uploadMutation.isPending || !can.edit}
              onClick={() => fileInputRef.current?.click()}
            >
              <Upload className="h-4 w-4" />
            </Button>
          </div>
          {/* 隐藏的文件输入元素 */}
          <input
            ref={fileInputRef}
            type="file"
            multiple
            className="hidden"
            onChange={(e) => {
              const files = Array.from(e.target.files || []);
              if (files.length > 0) {
                handleUpload(files);
                // 清空 input 以允许重复上传同一文件
                e.target.value = "";
              }
            }}
          />
        </div>

        <div className="flex items-center justify-between border-b bg-muted/50 px-4 py-2">
          <div className="text-sm text-muted-foreground">
            {selectedCount > 0
              ? `已选择 ${selectedCount} 个文件`
              : `共 ${files.length} 个文件`}
          </div>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => (allSelected
                ? clearFileSelection()
                : selectAllFiles(fileIds))}
              title={allSelected ? "取消全选" : "全选"}
              disabled={files.length === 0}
            >
              {allSelected
                ? <Square className="h-3 w-3" />
                : <CheckSquare className="h-3 w-3" />}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => invertFileSelection(fileIds)}
              disabled={files.length === 0}
              title="反选"
            >
              <Repeat className="h-3 w-3" />
            </Button>
          </div>
        </div>

        {/* Content */}
        <ScrollArea className="flex-1 h-0">
          <div className="p-2 space-y-4">
            {isLoading
              ? (
                <div className="text-center text-sm text-muted-foreground">
                  加载中...
                </div>
              )
              : (
                <FileListView
                  files={files}
                  onDelete={handleDelete}
                  onFileClick={handleFileClick}
                  showCheckboxes={true}
                />
              )}
          </div>
        </ScrollArea>

        <div className="border-t">
          {/* Download Button */}
          {selectedCount > 0 && (
            <div className="p-4 pb-2">
              <Button
                className="w-full relative"
                onClick={handleBatchDownload}
                disabled={selectedCount === 0}
              >
                <Download className="mr-2 h-4 w-4" />
                下载选中文件
                <Badge
                  variant="secondary"
                  className="ml-2 bg-primary-foreground text-primary"
                >
                  {selectedCount}
                </Badge>
              </Button>
            </div>
          )}

          {/* Upload Zone */}
          {can.edit && (
            <div className="p-4 pt-2">
              <FileUploadZone
                onUpload={handleUpload}
                isUploading={activeUploads > 0}
              />
            </div>
          )}
        </div>
      </aside>

      <FilePreviewModal
        file={previewFile}
        open={previewOpen}
        onOpenChange={setPreviewOpen}
        onDelete={handleDelete}
      />

      <UrlUploadDialog
        open={urlDialogOpen}
        onOpenChange={setUrlDialogOpen}
        onSubmit={(data) => uploadUrlMutation.mutate(data)}
        isUploading={uploadUrlMutation.isPending}
      />
    </>
  );
}
