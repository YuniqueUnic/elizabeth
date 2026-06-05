/**
 * File Management Service
 */

import { API_BASE_URL, API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getAccessToken, getValidToken } from "./authService";
import type { TransferProgress } from "../lib/transfer-types";
import type {
  BackendRoomContent,
  FileItem,
  UpdateContentResponse,
  UploadContentResponse,
  UploadPreparationRequest,
  UploadPreparationResponse,
} from "../lib/types";
import {
  backendContentToFileItem as convertFile,
  ContentType as CT,
  parseContentType,
} from "../lib/types";

export interface UploadUrlRequest {
  url: string;
  name: string;
  description?: string;
}

const CHUNKED_UPLOAD_THRESHOLD =
  typeof window !== "undefined" &&
  (window as any).__CHUNKED_UPLOAD_THRESHOLD !== undefined
    ? (window as any).__CHUNKED_UPLOAD_THRESHOLD
    : 5 * 1024 * 1024; // 5MB

async function ensureToken(roomName: string, token?: string): Promise<string> {
  if (token) return token;
  const existing = await getValidToken(roomName);
  if (existing) return existing;
  const tokenResponse = await getAccessToken(roomName);
  return tokenResponse.token;
}

function buildAuthenticatedDownloadPath(fileId: string, token: string): string {
  const url = new URL(`${API_BASE_URL}/contents/${fileId}`, "http://elizabeth.local");
  url.searchParams.set("token", token);
  return `${url.pathname}${url.search}${url.hash}`;
}

/** Return the content path without token (for Authorization header usage) */
function getContentPath(fileId: string): string {
  return `${API_BASE_URL}/contents/${fileId}`;
}

function triggerBrowserDownload(blob: Blob, fileName: string): void {
  const objectUrl = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = objectUrl;
  link.download = fileName;
  link.rel = "noopener";
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
}

export async function getDownloadUrl(
  roomName: string,
  fileId: string,
  token?: string,
): Promise<string> {
  const authToken = await ensureToken(roomName, token);
  return buildAuthenticatedDownloadPath(fileId, authToken);
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
  return contents
    .filter((content) => {
      const contentType = parseContentType(content.content_type);
      return contentType !== CT.Text;
    })
    .map((content) => convertFile(content, roomName))
    .sort((a, b) =>
      new Date(b.uploadedAt || "").getTime() -
      new Date(a.uploadedAt || "").getTime()
    );
}

/**
 * Upload a file with progress tracking and abort support.
 * Uses XHR for real upload progress events.
 */
export async function uploadFile(
  roomName: string,
  file: File,
  options?: {
    token?: string;
    abortSignal?: AbortSignal;
    onProgress?: (progress: TransferProgress) => void;
  },
): Promise<FileItem> {
  const authToken = await ensureToken(roomName, options?.token);
  const useChunkedUpload = file.size > CHUNKED_UPLOAD_THRESHOLD;

  if (useChunkedUpload) {
    const { uploadFileChunked } = await import("./chunkedUploadService");
    const mergeResponse = await uploadFileChunked(
      roomName,
      file,
      {
        onProgress: options?.onProgress,
        abortSignal: options?.abortSignal,
      },
      authToken,
    );
    const mergedFile = mergeResponse.merged_files?.[0];
    if (!mergedFile) throw new Error("No merged files in response");
    if (mergedFile.content_id == null) throw new Error("Merged file missing content_id");

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
      url: `/contents/${mergedFile.content_id}`,
      assetUrl: `${API_BASE_URL}/contents/${mergedFile.content_id}`,
    };
  }

  // Simple upload with XHR for progress
  const prepareRequest: UploadPreparationRequest = {
    files: [{ name: file.name, size: file.size, mime: file.type }],
  };

  const prepareResponse = await api.post<UploadPreparationResponse>(
    API_ENDPOINTS.content.prepare(roomName),
    prepareRequest,
    { token: authToken },
  );

  const formData = new FormData();
  formData.append("file", file);

  const uploadedContents = await xhrUpload<UploadContentResponse>(
    `${API_BASE_URL}${API_ENDPOINTS.content.base(roomName)}?reservation_id=${prepareResponse.reservation_id}`,
    formData,
    authToken,
    options?.abortSignal,
    options?.onProgress,
  );

  const uploaded = uploadedContents.uploaded ?? [];
  if (uploaded.length === 0) throw new Error("Failed to upload file");
  return convertFile(uploaded[0]);
}

/** XHR-based upload with progress events */
function xhrUpload<T>(
  url: string,
  formData: FormData,
  token: string,
  abortSignal?: AbortSignal,
  onProgress?: (progress: TransferProgress) => void,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", url);
    xhr.setRequestHeader("Authorization", `Bearer ${token}`);

    let lastLoaded = 0;
    let lastTime = Date.now();

    xhr.upload.onprogress = (e) => {
      if (!e.lengthComputable || !onProgress) return;
      const now = Date.now();
      const dt = (now - lastTime) / 1000;
      const dB = e.loaded - lastLoaded;
      const speed = dt > 0 ? dB / dt : 0;
      const remaining = speed > 0 ? (e.total - e.loaded) / speed : 0;
      lastLoaded = e.loaded;
      lastTime = now;
      onProgress({
        bytesTransferred: e.loaded,
        totalBytes: e.total,
        percentage: (e.loaded / e.total) * 100,
        speed,
        estimatedTimeRemaining: remaining,
      });
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try { resolve(JSON.parse(xhr.responseText)); }
        catch { reject(new Error("Invalid JSON response")); }
      } else {
        reject(new Error(`Upload failed: ${xhr.status}`));
      }
    };
    xhr.onerror = () => reject(new Error("Upload failed"));
    xhr.onabort = () => reject(new DOMException("Aborted", "AbortError"));

    abortSignal?.addEventListener("abort", () => xhr.abort());
    xhr.send(formData);
  });
}

/**
 * Delete a file
 */
export async function deleteFile(
  roomName: string,
  fileId: string,
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);
  const idsParam = parseInt(fileId, 10);
  await api.delete(
    `${API_ENDPOINTS.content.base(roomName)}?ids=${idsParam}`,
    { ids: [idsParam] },
    { token: authToken },
  );
}

/**
 * Delete multiple files
 */
export async function deleteFiles(
  roomName: string,
  fileIds: string[],
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);
  const numericIds = fileIds.map((id) => parseInt(id, 10));
  const idsParam = numericIds.join(",");
  await api.delete(
    `${API_ENDPOINTS.content.base(roomName)}?ids=${idsParam}`,
    { ids: numericIds },
    { token: authToken },
  );
}

/**
 * Download a file with progress tracking and abort support.
 * Uses XHR for real download progress events.
 */
export async function downloadFile(
  roomName: string,
  fileId: string,
  fileName: string,
  token?: string,
  options?: {
    abortSignal?: AbortSignal;
    onProgress?: (progress: TransferProgress) => void;
  },
): Promise<void> {
  const authToken = await ensureToken(roomName, token);
  const downloadPath = getContentPath(fileId);

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.responseType = "blob";
    xhr.open("GET", downloadPath);
    xhr.setRequestHeader("Authorization", `Bearer ${authToken}`);

    let lastLoaded = 0;
    let lastTime = Date.now();

    xhr.onprogress = (e) => {
      if (!e.lengthComputable || !options?.onProgress) return;
      const now = Date.now();
      const dt = (now - lastTime) / 1000;
      const dB = e.loaded - lastLoaded;
      const speed = dt > 0 ? dB / dt : 0;
      const remaining = speed > 0 ? (e.total - e.loaded) / speed : 0;
      lastLoaded = e.loaded;
      lastTime = now;
      options.onProgress({
        bytesTransferred: e.loaded,
        totalBytes: e.total,
        percentage: (e.loaded / e.total) * 100,
        speed,
        estimatedTimeRemaining: remaining,
      });
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        triggerBrowserDownload(xhr.response as Blob, fileName);
        resolve();
      } else {
        reject(new Error(`Download failed: ${xhr.status}`));
      }
    };
    xhr.onerror = () => reject(new Error("Download failed"));
    xhr.onabort = () => reject(new DOMException("Aborted", "AbortError"));

    options?.abortSignal?.addEventListener("abort", () => xhr.abort());
    xhr.send();
  });
}

/**
 * Cancel a chunked upload, cleaning up server-side temp files.
 */
export async function cancelUpload(
  roomName: string,
  reservationId: string,
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);
  await api.delete(
    API_ENDPOINTS.chunkedUpload.cancel(roomName, reservationId),
    undefined,
    { token: authToken },
  );
}

/**
 * Batch download files one by one
 */
export async function downloadFilesBatch(
  roomName: string,
  fileIds: string[],
  token?: string,
): Promise<void> {
  const authToken = await ensureToken(roomName, token);
  const files = await getFilesList(roomName, authToken);
  const fileMap = new Map(files.map((f) => [f.id, f.name]));

  for (const fileId of fileIds) {
    try {
      const fileName = fileMap.get(fileId) || `file_${fileId}`;
      await downloadFile(roomName, fileId, fileName, authToken);
      if (fileIds.indexOf(fileId) < fileIds.length - 1) {
        await new Promise((resolve) => setTimeout(resolve, 200));
      }
    } catch (error) {
      console.error(`Failed to download file ${fileId}:`, error);
    }
  }
}

/**
 * Upload a URL as content to a room
 */
export async function uploadUrl(
  roomName: string,
  data: UploadUrlRequest,
  token?: string,
): Promise<FileItem> {
  const authToken = await ensureToken(roomName, token);
  const placeholderText = `URL: ${data.url}`;
  const placeholderBlob = new Blob([placeholderText], { type: "text/plain" });
  const placeholderFile = new File([placeholderBlob], data.name, { type: "text/plain" });

  const uploadedContent = await uploadFile(roomName, placeholderFile, { token: authToken });

  const updateResponse = await api.put<UpdateContentResponse>(
    `${API_ENDPOINTS.content.byId(roomName, uploadedContent.id)}`,
    { url: data.url, mime_type: data.description || "text/html" },
    { token: authToken },
  );
  return convertFile(updateResponse.updated, roomName);
}

const fileService = {
  getFilesList,
  uploadFile,
  uploadUrl,
  deleteFile,
  deleteFiles,
  downloadFile,
  downloadFilesBatch,
  cancelUpload,
};

export default fileService;
