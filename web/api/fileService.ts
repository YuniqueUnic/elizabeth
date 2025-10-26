// Mock API service for file operations
import type { FileItem } from "@/lib/types"

// Mock: Get files list
export const getFilesList = async (roomId: string): Promise<FileItem[]> => {
  await new Promise((resolve) => setTimeout(resolve, 300))

  return [
    {
      id: "file_1",
      name: "project-proposal.pdf",
      thumbnailUrl: null,
      size: 2048000, // 2MB
      type: "pdf",
      url: "https://example.com/files/project-proposal.pdf",
    },
    {
      id: "file_2",
      name: "design-mockup.png",
      thumbnailUrl: "/placeholder.svg?height=100&width=100",
      size: 1536000, // 1.5MB
      type: "image",
      url: "/placeholder.svg?height=800&width=1200",
    },
    {
      id: "file_3",
      name: "meeting-notes.docx",
      thumbnailUrl: null,
      size: 512000, // 0.5MB
      type: "document",
      url: "https://example.com/files/meeting-notes.docx",
    },
    {
      id: "file_4",
      name: "demo-video.mp4",
      thumbnailUrl: "/placeholder.svg?height=100&width=100",
      size: 5242880, // 5MB
      type: "video",
      url: "https://example.com/files/demo-video.mp4",
    },
    {
      id: "file_5",
      name: "External Link",
      thumbnailUrl: null,
      size: 0,
      type: "link",
      url: "https://github.com/vercel/next.js",
    },
  ]
}

// Mock: Upload file
export const uploadFile = async (roomId: string, file: File): Promise<FileItem> => {
  console.log("[API Mock] uploadFile:", { roomId, fileName: file.name, size: file.size })

  // Simulate upload delay
  await new Promise((resolve) => setTimeout(resolve, 1000))

  let fileType: FileItem["type"] = "document"
  if (file.type.startsWith("image/")) {
    fileType = "image"
  } else if (file.type.startsWith("video/")) {
    fileType = "video"
  } else if (file.name.endsWith(".pdf")) {
    fileType = "pdf"
  }

  return {
    id: `file_${Date.now()}`,
    name: file.name,
    thumbnailUrl: file.type.startsWith("image/") ? "/placeholder.svg?height=100&width=100" : null,
    size: file.size,
    type: fileType,
    url: URL.createObjectURL(file),
  }
}

// Mock: Delete file
export const deleteFile = async (fileId: string): Promise<void> => {
  console.log("[API Mock] deleteFile:", fileId)
  await new Promise((resolve) => setTimeout(resolve, 300))
}

// Mock: Batch download files
export const downloadFilesBatch = async (fileIds: string[]): Promise<void> => {
  console.log("[API Mock] downloadFilesBatch:", fileIds)

  // Simulate download preparation
  await new Promise((resolve) => setTimeout(resolve, 500))

  // Create a mock download
  const link = document.createElement("a")
  link.href = "data:text/plain;charset=utf-8,Mock%20Batch%20Download%20Content"
  link.download = `elizabeth_batch_${Date.now()}.zip`
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}
