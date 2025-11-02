/**
 * File Management Service
 *
 * This service handles file-related operations including:
 * - Listing files in a room
 * - Uploading files with chunked upload support
 * - Deleting files
 * - Batch downloading files
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getAccessToken, getValidToken } from "./authService";
import type { BackendRoomContent, FileItem } from "../lib/types";
import {
  backendContentToFileItem as convertFile,
  ContentType as CT,
  parseContentType,
} from "../lib/types";

// ============================================================================
// File Request/Response Types
// ============================================================================

export interface PrepareUploadRequest {
  files: Array<{
    name: string;
    size: number;
    mime: string;
  }>;
}

export interface PrepareUploadResponse {
  reservation_id: string;
  reservations: Array<{
    file_name: string;
    expected_size: number;
    mime_type: string;
  }>;
}

// ============================================================================
// File Functions
// ============================================================================

const CHUNKED_UPLOAD_THRESHOLD = 5 * 1024 * 1024; // 5MB

async function ensureToken(roomName: string, token?: string): Promise<string> {
  if (token) return token;

  const existing = await getValidToken(roomName);
  if (existing) return existing;

  const tokenResponse = await getAccessToken(roomName);
  return tokenResponse.token;
}

function mapMimeToFileType(mime?: string): FileItem["type"] {
  if (!mime) return "document";
  if (mime.startsWith("image/")) return "image";
  if (mime.startsWith("video/")) return "video";
  if (mime === "application/pdf") return "pdf";
  if (mime.startsWith("http")) return "link";
  return "document";
}

/**
 * Get all files for a room
 * Filters RoomContent for non-text content (content_type != 0)
 *
 * @param roomName - The name of the room
 * @param token - Optional token for authentication
 * @returns Array of files
 */
export async function getFilesList(
  roomName: string,
  token?: string,
): Promise<FileItem[]> {
  const authToken = await ensureToken(roomName, token);

  const contents = await api.get<BackendRoomContent[]>(
    API_ENDPOINTS.content.base(roomName),
    undefined,
    { token: authToken },
  );

  // Filter for non-text content (files only) and convert to FileItem
  // Exclude text messages (ContentType.Text or text/plain files with message.txt pattern)
  const filtered = contents
    .filter((content) => {
      const contentType = parseContentType(content.content_type);

      // Exclude explicit text content
      if (contentType === CT.Text) {
        return false;
      }
      // Exclude text files that are messages (mime_type is text/plain and filename includes message.txt)
      if (
        contentType === CT.File &&
        content.mime_type === "text/plain" &&
        content.file_name?.includes("message.txt")
      ) {
        return false;
      }
      return true;
    })
    .map((content) => convertFile(content, roomName)) // ✅ FIX: Pass roomName to generate download URLs
    .sort((a, b) =>
      new Date(b.uploadedAt || "").getTime() -
      new Date(a.uploadedAt || "").getTime()
    );

  return filtered;
}

/**
 * Upload a file to a room
 *
 * For small files (<5MB), uses the simple upload process.
 * For large files, automatically switches to chunked upload.
 *
 * @param roomName - The name of the room
 * @param file - The file to upload
 * @param token - Optional token for authentication
 * @param options - Upload options including progress callbacks
 * @returns The uploaded file item
 */
export async function uploadFile(
  roomName: string,
  file: File,
  token?: string,
  options?: {
    useChunkedUpload?: boolean;
    onProgress?: (progress: {
      bytesUploaded: number;
      totalBytes: number;
      percentage: number;
    }) => void;
  },
): Promise<FileItem> {
  const authToken = await ensureToken(roomName, token);
  const useChunkedUpload = options?.useChunkedUpload ??
    file.size > CHUNKED_UPLOAD_THRESHOLD;

  if (useChunkedUpload) {
    const { uploadFileChunked } = await import("./chunkedUploadService");
    const mergeResponse = await uploadFileChunked(
      roomName,
      file,
      {
        onProgress: (progress) => options?.onProgress?.(progress),
      },
      authToken,
    );

    const mergedFile = mergeResponse.merged_files?.[0];
    if (!mergedFile) {
      throw new Error("No merged files in response");
    }

    const timestamp = new Date().toISOString();
    return {
      id: mergedFile.content_id.toString(),
      name: mergedFile.file_name,
      thumbnailUrl: null,
      size: mergedFile.file_size,
      type: mapMimeToFileType(file.type),
      mimeType: file.type || undefined,
      createdAt: timestamp,
      uploadedAt: timestamp,
      url: `${API_ENDPOINTS.content.base(roomName)}/${mergedFile.content_id}`,
    };
  }

  const prepareRequest: PrepareUploadRequest = {
    files: [{
      name: file.name,
      size: file.size,
      mime: file.type,
    }],
  };

  const prepareResponse = await api.post<PrepareUploadResponse>(
    API_ENDPOINTS.content.prepare(roomName),
    prepareRequest,
    { token: authToken },
  );

  const formData = new FormData();
  formData.append("file", file);

  const uploadOnce = async (reservationId: string) => {
    const response = await api.post<
      { uploaded: BackendRoomContent[]; current_size: number }
    >(
      `${API_ENDPOINTS.content.base(roomName)}?reservation_id=${reservationId}`,
      formData,
      { token: authToken },
    );
    return response.uploaded ?? [];
  };

  let uploadedContents: BackendRoomContent[];
  try {
    uploadedContents = await uploadOnce(prepareResponse.reservation_id);
  } catch (err: any) {
    if (err?.code !== 400) {
      throw err;
    }
    const retryPrepare = await api.post<PrepareUploadResponse>(
      API_ENDPOINTS.content.prepare(roomName),
      prepareRequest,
      { token: authToken },
    );
    uploadedContents = await uploadOnce(retryPrepare.reservation_id);
  }

  if (uploadedContents.length === 0) {
    throw new Error("Failed to upload file");
  }

  return convertFile(uploadedContents[0]);
}

/**
 * Delete a file
 *
 * @param roomName - The name of the room
 * @param fileId - The ID of the file to delete
 * @param token - Optional token for authentication
 */
export async function deleteFile(
  roomName: string,
  fileId: string,
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);

  // Backend expects token in query parameter AND ids in both query and body
  const idsParam = parseInt(fileId, 10);
  await api.delete(
    `${
      API_ENDPOINTS.content.base(roomName)
    }?ids=${idsParam}&token=${authToken}`,
    { ids: [idsParam] },
  );
}

/**
 * Delete multiple files
 *
 * @param roomName - The name of the room
 * @param fileIds - Array of file IDs to delete
 * @param token - Optional token for authentication
 */
export async function deleteFiles(
  roomName: string,
  fileIds: string[],
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);

  const numericIds = fileIds.map((id) => parseInt(id, 10));
  const idsParam = numericIds.join(",");
  // Backend expects token in query parameter AND ids in both query and body
  await api.delete(
    `${
      API_ENDPOINTS.content.base(roomName)
    }?ids=${idsParam}&token=${authToken}`,
    { ids: numericIds },
  );
}

/**
 * Download a single file
 *
 * @param roomName - The name of the room
 * @param fileId - The ID of the file to download
 * @param fileName - The name of the file to download
 * @param token - Optional token for authentication
 * @returns Promise that resolves when download is triggered
 */
export async function downloadFile(
  roomName: string,
  fileId: string,
  fileName: string,
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);

  const response = await api.get(
    `${API_ENDPOINTS.content.base(roomName)}/${fileId}`,
    undefined,
    {
      token: authToken,
      responseType: "blob", // Important for file downloads
    },
  );

  // Create download link for the file
  const blob = new Blob([response]);
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  window.URL.revokeObjectURL(url);
}

/**
 * Batch download files
 * Downloads multiple files one by one (backend doesn't support ZIP yet)
 *
 * @param roomName - The name of the room
 * @param fileIds - Array of file IDs to download
 * @param token - Optional token for authentication
 * @returns Promise that resolves when all downloads are triggered
 */
export async function downloadFilesBatch(
  roomName: string,
  fileIds: string[],
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);

  // ✅ FIX: Get file list to retrieve file names
  const files = await getFilesList(roomName, authToken);
  const fileMap = new Map(files.map((f) => [f.id, f.name]));

  // Download files one by one instead of creating a ZIP
  for (const fileId of fileIds) {
    try {
      const fileName = fileMap.get(fileId) || `file_${fileId}`;

      // Use the downloadFile function which properly handles file names
      await downloadFile(roomName, fileId, fileName, authToken);

      // Small delay between downloads to avoid overwhelming the browser
      if (fileIds.indexOf(fileId) < fileIds.length - 1) {
        await new Promise((resolve) => setTimeout(resolve, 200));
      }
    } catch (error) {
      console.error(`Failed to download file ${fileId}:`, error);
      // Continue with other files even if one fails
    }
  }
}

// Legacy compatibility exports (for existing components)
// getFilesList is already exported above
export default {
  getFilesList,
  uploadFile,
  deleteFile,
  deleteFiles,
  downloadFile,
  downloadFilesBatch,
};
