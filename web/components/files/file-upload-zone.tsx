"use client"

import { useCallback } from "react"
import { useDropzone } from "react-dropzone"
import { Upload } from "lucide-react"
import { cn } from "@/lib/utils"

interface FileUploadZoneProps {
  onUpload: (files: File[]) => void
  isUploading: boolean
}

export function FileUploadZone({ onUpload, isUploading }: FileUploadZoneProps) {
  const onDrop = useCallback(
    (acceptedFiles: File[]) => {
      if (!isUploading) {
        onUpload(acceptedFiles)
      }
    },
    [onUpload, isUploading],
  )

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    disabled: isUploading,
  })

  return (
    <div
      {...getRootProps()}
      className={cn(
        "flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed p-6 transition-colors",
        isDragActive && "border-primary bg-primary/5",
        isUploading && "cursor-not-allowed opacity-50",
        !isDragActive && !isUploading && "hover:border-primary/50 hover:bg-accent/50",
      )}
    >
      <input {...getInputProps()} />
      <Upload className="mb-2 h-8 w-8 text-muted-foreground" />
      <p className="text-center text-sm text-muted-foreground">
        {isUploading ? "上传中..." : isDragActive ? "释放以上传文件" : "拖拽文件到此处或点击上传"}
      </p>
    </div>
  )
}
