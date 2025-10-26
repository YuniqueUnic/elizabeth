"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { FileListView } from "@/components/files/file-list-view"
import { FileUploadZone } from "@/components/files/file-upload-zone"
import { FilePreviewModal } from "@/components/files/file-preview-modal"
import { Upload, Download, CheckSquare, Square, Repeat } from "lucide-react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { getFilesList, uploadFile, deleteFile, downloadFilesBatch } from "@/api/fileService"
import { useAppStore } from "@/lib/store"
import { useToast } from "@/hooks/use-toast"
import type { FileItem } from "@/lib/types"

export function RightSidebar() {
  const currentRoomId = useAppStore((state) => state.currentRoomId)
  const selectedFiles = useAppStore((state) => state.selectedFiles)
  const clearFileSelection = useAppStore((state) => state.clearFileSelection)
  const selectAllFiles = useAppStore((state) => state.selectAllFiles)
  const invertFileSelection = useAppStore((state) => state.invertFileSelection)
  const queryClient = useQueryClient()
  const { toast } = useToast()

  const [previewFile, setPreviewFile] = useState<FileItem | null>(null)
  const [previewOpen, setPreviewOpen] = useState(false)

  const { data: files = [], isLoading } = useQuery({
    queryKey: ["files", currentRoomId],
    queryFn: () => getFilesList(currentRoomId),
  })

  const uploadMutation = useMutation({
    mutationFn: (file: File) => uploadFile(currentRoomId, file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["files", currentRoomId] })
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] })
      toast({
        title: "上传成功",
        description: "文件已成功上传到房间",
      })
    },
    onError: () => {
      toast({
        title: "上传失败",
        description: "文件上传失败，请重试",
        variant: "destructive",
      })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (fileId: string) => deleteFile(fileId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["files", currentRoomId] })
      queryClient.invalidateQueries({ queryKey: ["room", currentRoomId] })
      toast({
        title: "删除成功",
        description: "文件已从房间中删除",
      })
    },
    onError: () => {
      toast({
        title: "删除失败",
        description: "无法删除文件，请重试",
        variant: "destructive",
      })
    },
  })

  const handleUpload = async (acceptedFiles: File[]) => {
    for (const file of acceptedFiles) {
      await uploadMutation.mutateAsync(file)
    }
  }

  const handleDelete = (fileId: string) => {
    deleteMutation.mutate(fileId)
  }

  const handleBatchDownload = async () => {
    if (selectedFiles.size > 0) {
      toast({
        title: "开始下载",
        description: `正在准备下载 ${selectedFiles.size} 个文件`,
      })
      await downloadFilesBatch(Array.from(selectedFiles))
      clearFileSelection()
      toast({
        title: "下载完成",
        description: "文件已成功下载",
      })
    }
  }

  const handleFileClick = (file: FileItem) => {
    setPreviewFile(file)
    setPreviewOpen(true)
  }

  const selectedCount = selectedFiles.size
  const fileIds = files.map((f) => f.id)
  const allSelected = selectedFiles.size === files.length && files.length > 0

  return (
    <>
      <aside className="flex w-80 flex-col border-l bg-muted/30">
        {/* Header */}
        <div className="flex h-12 items-center justify-between border-b px-4">
          <h2 className="font-semibold">文件管理</h2>
          <Button variant="ghost" size="icon" title="上传文件" disabled={uploadMutation.isPending}>
            <Upload className="h-4 w-4" />
          </Button>
        </div>

        <div className="flex items-center justify-between border-b bg-muted/50 px-4 py-2">
          <div className="text-sm text-muted-foreground">
            {selectedCount > 0 ? `已选择 ${selectedCount} 个文件` : `共 ${files.length} 个文件`}
          </div>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => (allSelected ? clearFileSelection() : selectAllFiles(fileIds))}
              title={allSelected ? "取消全选" : "全选"}
              disabled={files.length === 0}
            >
              {allSelected ? <Square className="h-3 w-3" /> : <CheckSquare className="h-3 w-3" />}
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
        <ScrollArea className="flex-1">
          <div className="p-4 space-y-4">
            {isLoading ? (
              <div className="text-center text-sm text-muted-foreground">加载中...</div>
            ) : (
              <FileListView files={files} onDelete={handleDelete} onFileClick={handleFileClick} showCheckboxes={true} />
            )}
          </div>
        </ScrollArea>

        <div className="border-t">
          {/* Download Button */}
          {selectedCount > 0 && (
            <div className="p-4 pb-2">
              <Button className="w-full relative" onClick={handleBatchDownload} disabled={selectedCount === 0}>
                <Download className="mr-2 h-4 w-4" />
                下载选中文件
                <Badge variant="secondary" className="ml-2 bg-primary-foreground text-primary">
                  {selectedCount}
                </Badge>
              </Button>
            </div>
          )}

          {/* Upload Zone */}
          <div className="p-4 pt-2">
            <FileUploadZone onUpload={handleUpload} isUploading={uploadMutation.isPending} />
          </div>
        </div>
      </aside>

      <FilePreviewModal file={previewFile} open={previewOpen} onOpenChange={setPreviewOpen} onDelete={handleDelete} />
    </>
  )
}
