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
  let authToken = token || await getValidToken(roomName);

  if (!authToken) {
    // Token not available, try to get a new one
    try {
      const tokenResponse = await getAccessToken(roomName);
      authToken = tokenResponse.token;
    } catch (err) {
      throw new Error("Authentication required to get files");
    }
  }

  if (!authToken) {
    throw new Error("Authentication required to get files");
  }

  const contents = await api.get<BackendRoomContent[]>(
    API_ENDPOINTS.content.base(roomName),
    undefined,
    { token: authToken },
  );

  console.log(`[getFilesList] 查询 ${roomName}: 返回 ${contents.length}个内容`);

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
    .map(convertFile)
    .sort((a, b) =>
      new Date(b.uploadedAt || "").getTime() -
      new Date(a.uploadedAt || "").getTime()
    );

  console.log(`[getFilesList] 过滤后 ${filtered.length}个文件`);
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
  console.log(`[uploadFile] 开始上传：${file.name} (${file.size} bytes)`);

  const CHUNKED_UPLOAD_THRESHOLD = 5 * 1024 * 1024; // 5MB
  const shouldUseChunkedUpload = file.size > CHUNKED_UPLOAD_THRESHOLD;

  if (shouldUseChunkedUpload) {
    // Use chunked upload for large files
    console.log(`[uploadFile] 使用 chunked 上传`);
    const { uploadFileChunked } = await import("./chunkedUploadService");

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

    const mergeResponse = await uploadFileChunked(
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
      authToken,
    );

    console.log(`[uploadFile] 文件合并完成，response:`, mergeResponse);

    // Extract the merged file info from the response
    if (
      mergeResponse.merged_files && mergeResponse.merged_files.length > 0
    ) {
      const mergedFileInfo = mergeResponse.merged_files[0];

      // Convert to FileItem format
      const result: FileItem = {
        id: mergedFileInfo.content_id.toString(),
        name: mergedFileInfo.file_name,
        size: mergedFileInfo.file_size,
        type: "application/octet-stream",
        createdAt: new Date().toISOString(),
        url: `${
          API_ENDPOINTS.content.base(roomName)
        }/${mergedFileInfo.content_id}`,
      };

      console.log(`[uploadFile] 完成：返回 FileItem id=${result.id}`);
      return result;
    } else {
      throw new Error("No merged files in response");
    }
  } else {
    // Use simple upload for small files
    console.log(`[uploadFile] 使用 simple 上传，开始步骤...`);
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
    console.log(`[uploadFile] Step 1: 准备上传...`);
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
    console.log(
      `[uploadFile] Step 1 完成：reservation_id=${prepareResponse.reservation_id}`,
    );

    // Step 2: Upload file data
    console.log(`[uploadFile] Step 2: 上传文件数据...`);
    const formData = new FormData();
    formData.append("file", file);

    // Attempt upload; if reservation 过期 (400) 则自动重新预留并重试一次
    const doUpload = async (reservationId: string) => {
      const response = await api.post<
        { uploaded: BackendRoomContent[]; current_size: number }
      >(
        `${
          API_ENDPOINTS.content.base(roomName)
        }?reservation_id=${reservationId}`,
        formData,
        { token: authToken },
      );
      // 后端返回 {uploaded: [...], current_size: N}
      return response.uploaded || [];
    };

    let uploadedContents: BackendRoomContent[];
    try {
      uploadedContents = await doUpload(prepareResponse.reservation_id);
      console.log(
        `[uploadFile] 上传成功，返回 ${uploadedContents.length}个内容`,
      );
      console.log(`[uploadFile] 返回数据:`, uploadedContents);
    } catch (err: any) {
      console.error(`[uploadFile] 上传失败:`, err);
      // If Bad Request, likely reservation TTL expired; re-prepare once
      if (err?.code === 400) {
        console.log(`[uploadFile] 重试：reservation 过期，重新准备...`);
        const retryPrepare = await api.post<PrepareUploadResponse>(
          API_ENDPOINTS.content.prepare(roomName),
          prepareRequest,
          { token: authToken },
        );
        uploadedContents = await doUpload(retryPrepare.reservation_id);
        console.log(
          `[uploadFile] 重试成功，返回 ${uploadedContents.length}个内容`,
        );
      } else {
        throw err;
      }
    }

    if (uploadedContents.length === 0) {
      throw new Error("Failed to upload file");
    }

    const result = convertFile(uploadedContents[0]);
    console.log(`[uploadFile] 完成：返回 FileItem id=${result.id}`);
    return result;
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
