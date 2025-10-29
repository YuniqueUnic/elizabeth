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
import { getValidToken } from "./authService";
import type {
  backendContentToFileItem,
  BackendRoomContent,
  ContentType,
  FileItem,
} from "../lib/types";
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

export interface ChunkedUploadRequest {
  reservation_id: string;
  chunk_index: number;
  data: ArrayBuffer;
  is_last: boolean;
}

// ============================================================================
// File Functions
// ============================================================================

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
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to get files");
  }

  const contents = await api.get<BackendRoomContent[]>(
    API_ENDPOINTS.content.base(roomName),
    undefined,
    { token: authToken },
  );

  // Filter for non-text content (files only) and convert to FileItem
  // Exclude text messages (ContentType.Text or text/plain files with message.txt pattern)
  return contents
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
    .map(convertFile)
    .sort((a, b) =>
      new Date(b.uploadedAt || "").getTime() -
      new Date(a.uploadedAt || "").getTime()
    );
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
  const CHUNKED_UPLOAD_THRESHOLD = 5 * 1024 * 1024; // 5MB
  const shouldUseChunkedUpload = options?.useChunkedUpload ??
    (file.size > CHUNKED_UPLOAD_THRESHOLD);

  if (shouldUseChunkedUpload) {
    // Use chunked upload for large files
    const { uploadFileChunked } = await import("./chunkedUploadService");

    let uploadedContent: BackendRoomContent | null = null;

    await uploadFileChunked(
      roomName,
      file,
      {
        onProgress: (progress) => {
          options?.onProgress?.(progress);
        },
        onChunkComplete: () => {
          // For now, we don't get intermediate results from chunked upload
          // In a real implementation, you might want to track chunk completion
        },
      },
      token,
    );

    // After chunked upload is complete, we need to get the uploaded content
    // This would typically be returned by the completeChunkedUpload endpoint
    // For now, we'll fall back to listing contents and finding the newest one
    const contents = await api.get<BackendRoomContent[]>(
      API_ENDPOINTS.content.base(roomName),
      undefined,
      { token },
    );

    // Find the most recently uploaded file
    const uploadedFiles = contents
      .filter((content) => parseContentType(content.content_type) !== CT.Text)
      .sort((a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );

    if (uploadedFiles.length === 0) {
      throw new Error("Failed to find uploaded file after chunked upload");
    }

    return convertFile(uploadedFiles[0]);
  } else {
    // Use simple upload for small files
    let authToken = token || await getValidToken(roomName);

    if (!authToken) {
      try {
        const { getAccessToken } = await import("./authService");
        const tokenResponse = await getAccessToken(roomName);
        authToken = tokenResponse.token;
      } catch (err) {
        console.error("Failed to get access token:", err);
        throw new Error("Authentication required to upload files");
      }
    }

    // Step 1: Prepare upload
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

    // Step 2: Upload file data
    const formData = new FormData();
    formData.append("file", file);

    // Attempt upload; if reservation 过期 (400) 则自动重新预留并重试一次
    const doUpload = async (reservationId: string) => {
      return await api.post<BackendRoomContent[]>(
        `${
          API_ENDPOINTS.content.base(roomName)
        }?reservation_id=${reservationId}`,
        formData,
        { token: authToken },
      );
    };

    let uploadedContents: BackendRoomContent[];
    try {
      uploadedContents = await doUpload(prepareResponse.reservation_id);
    } catch (err: any) {
      // If Bad Request, likely reservation TTL expired; re-prepare once
      if (err?.code === 400) {
        const retryPrepare = await api.post<PrepareUploadResponse>(
          API_ENDPOINTS.content.prepare(roomName),
          prepareRequest,
          { token: authToken },
        );
        uploadedContents = await doUpload(retryPrepare.reservation_id);
      } else {
        throw err;
      }
    }

    if (uploadedContents.length === 0) {
      throw new Error("Failed to upload file");
    }

    return convertFile(uploadedContents[0]);
  }
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
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to delete files");
  }

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
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("Authentication required to delete files");
  }

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
  let authToken = token || await getValidToken(roomName);

  if (!authToken) {
    try {
      const { getAccessToken } = await import("./authService");
      const tokenResponse = await getAccessToken(roomName);
      authToken = tokenResponse.token;
    } catch (err) {
      console.error("Failed to get access token:", err);
      throw new Error("Authentication required to download files");
    }
  }

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
 * Creates a ZIP file containing multiple files
 *
 * @param roomName - The name of the room
 * @param fileIds - Array of file IDs to download
 * @param token - Optional token for authentication
 * @returns Promise that resolves when download is triggered
 */
export async function downloadFilesBatch(
  roomName: string,
  fileIds: string[],
  token?: string,
): Promise<void> {
  let authToken = token || await getValidToken(roomName);

  if (!authToken) {
    try {
      const { getAccessToken } = await import("./authService");
      const tokenResponse = await getAccessToken(roomName);
      authToken = tokenResponse.token;
    } catch (err) {
      console.error("Failed to get access token:", err);
      throw new Error("Authentication required to download files");
    }
  }

  const response = await api.post(
    `${API_ENDPOINTS.content.base(roomName)}/download`,
    { file_ids: fileIds.map((id) => parseInt(id, 10)) },
    {
      token: authToken,
      responseType: "blob", // Important for file downloads
    },
  );

  // Create download link for the ZIP file
  const blob = new Blob([response], { type: "application/zip" });
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `elizabeth_files_${Date.now()}.zip`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  window.URL.revokeObjectURL(url);
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
