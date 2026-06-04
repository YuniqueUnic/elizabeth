/**
 * Chunked Upload Service
 *
 * This service handles large file uploads by splitting them into chunks
 * and uploading them individually with progress tracking.
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type { TransferProgress } from "../lib/transfer-types";
import type {
  ChunkedUploadPreparationRequest,
  ChunkedUploadPreparationResponse,
  FileMergeRequest,
  FileMergeResponse,
  UploadStatusQuery,
  UploadStatusResponse,
} from "../lib/types";

// ============================================================================
// Chunked Upload Types
// ============================================================================

export interface ChunkedUploadOptions {
  chunkSize?: number;
  maxRetries?: number;
  onProgress?: (progress: TransferProgress) => void;
  onChunkComplete?: (chunkIndex: number, totalChunks: number) => void;
  onError?: (error: Error, chunkIndex?: number) => void;
  abortSignal?: AbortSignal;
}

export interface ChunkData {
  chunk_index: number;
  data: ArrayBuffer;
  is_last: boolean;
}

// ============================================================================
// Chunked Upload Implementation
// ============================================================================

/**
 * Upload a file using chunked upload
 *
 * @param roomName - The name of the room
 * @param file - The file to upload
 * @param options - Upload options including progress callbacks
 * @param token - Optional authentication token
 * @returns Promise that resolves when upload is complete
 */
export async function uploadFileChunked(
  roomName: string,
  file: File,
  options: ChunkedUploadOptions = {},
  token?: string,
): Promise<FileMergeResponse> {
  const {
    chunkSize = 1024 * 1024,
    maxRetries = 3,
    onProgress,
    onChunkComplete,
    onError,
    abortSignal,
  } = options;

  const authToken = token || await getValidToken(roomName);
  if (!authToken) {
    throw new Error("Authentication required to upload files");
  }

  const totalChunks = Math.ceil(file.size / chunkSize);
  const startTime = Date.now();
  let bytesUploaded = 0;

  // Step 1: Prepare chunked upload
  const prepareRequest: ChunkedUploadPreparationRequest = {
    files: [{
      name: file.name,
      size: file.size,
      mime: file.type,
      chunk_size: chunkSize,
    }],
  };

  const prepareResponse = await api.post<ChunkedUploadPreparationResponse>(
    API_ENDPOINTS.chunkedUpload.prepare(roomName),
    prepareRequest,
    { token: authToken },
  );

  const uploadToken = prepareResponse.upload_token;
  const reservationId = prepareResponse.reservation_id;

  // Step 2: Upload chunks
  for (let chunkIndex = 0; chunkIndex < totalChunks; chunkIndex++) {
    // Check abort before each chunk
    if (abortSignal?.aborted) {
      throw new DOMException("Aborted", "AbortError");
    }

    const start = chunkIndex * chunkSize;
    const end = Math.min(start + chunkSize, file.size);
    const chunk = file.slice(start, end);

    try {
      const arrayBuffer = await chunk.arrayBuffer();
      const isLast = chunkIndex === totalChunks - 1;

      await uploadChunk(
        roomName,
        uploadToken,
        {
          chunk_index: chunkIndex,
          data: arrayBuffer,
          is_last: isLast,
        },
        authToken,
        maxRetries,
        abortSignal,
      );

      bytesUploaded += arrayBuffer.byteLength;
      const elapsed = (Date.now() - startTime) / 1000;
      const speed = bytesUploaded / elapsed;
      const remainingBytes = file.size - bytesUploaded;
      const estimatedTimeRemaining = speed > 0 ? remainingBytes / speed : 0;

      onProgress?.({
        bytesTransferred: bytesUploaded,
        totalBytes: file.size,
        percentage: (bytesUploaded / file.size) * 100,
        speed,
        estimatedTimeRemaining,
      });
      onChunkComplete?.(chunkIndex, totalChunks);
    } catch (error) {
      const chunkError = error instanceof Error
        ? error
        : new Error(String(error));
      onError?.(chunkError, chunkIndex);

      // On abort, try to cancel on server to clean up temp files
      if (abortSignal?.aborted || (error instanceof DOMException && error.name === "AbortError")) {
        try {
          const { cancelUpload } = await import("./fileService");
          await cancelUpload(roomName, reservationId, authToken).catch(() => {});
        } catch { /* ignore cleanup errors */ }
      }
      throw chunkError;
    }
  }

  // Calculate final file hash
  const fileHash = await calculateSHA256Hash(file);

  // Step 3: Complete the chunked upload by merging all chunks
  const completeRequest: FileMergeRequest = {
    reservation_id: reservationId,
    final_hash: fileHash,
  };

  const completeResponse = await api.post<FileMergeResponse>(
    API_ENDPOINTS.chunkedUpload.complete(roomName),
    completeRequest,
    { token: authToken },
  );

  return completeResponse;
}

/**
 * Upload a single chunk
 *
 * @param roomName - The name of the room
 * @param uploadToken - The upload token from prepare step
 * @param chunkData - The chunk data to upload
 * @param token - Authentication token
 * @param maxRetries - Maximum number of retries
 * @returns Promise that resolves when chunk is uploaded
 */
async function uploadChunk(
  roomName: string,
  uploadToken: string,
  chunkData: ChunkData,
  token: string,
  maxRetries: number = 3,
  abortSignal?: AbortSignal,
): Promise<void> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    if (abortSignal?.aborted) {
      throw new DOMException("Aborted", "AbortError");
    }

    try {
      const formData = new FormData();
      formData.append("upload_token", uploadToken);
      formData.append("chunk_index", chunkData.chunk_index.toString());
      formData.append("chunk_size", chunkData.data.byteLength.toString());

      const chunkBlob = new Blob([chunkData.data]);
      formData.append("chunk_data", chunkBlob);

      // Use fetch directly to support abort signal
      const url = `${API_ENDPOINTS.chunkedUpload.upload(roomName)}`;
      const response = await fetch(url, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        body: formData,
        signal: abortSignal,
      });

      if (!response.ok) {
        const err = new Error(`Chunk upload failed: ${response.status}`) as any;
        err.code = response.status;
        throw err;
      }

      return;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") throw error;
      lastError = error instanceof Error ? error : new Error(String(error));

      if (error && typeof error === "object" && "code" in error) {
        const errorCode = (error as any).code;
        if (typeof errorCode === "number" && errorCode >= 400 && errorCode < 500) {
          throw lastError;
        }
      }

      if (attempt < maxRetries) {
        const delay = Math.pow(2, attempt) * 1000;
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }

  throw lastError || new Error("Chunk upload failed after retries");
}

/**
 * Get the status of a chunked upload
 *
 * @param roomName - The name of the room
 * @param uploadId - The upload ID
 * @param token - Authentication token
 * @returns Upload status information
 */
export async function getChunkedUploadStatus(
  roomName: string,
  query: UploadStatusQuery,
  token?: string,
): Promise<UploadStatusResponse> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) {
    throw new Error("Authentication required to check upload status");
  }

  const params: Record<string, string | number | boolean> = {};
  if (query.upload_token) {
    params.upload_token = query.upload_token;
  }
  if (query.reservation_id) {
    params.reservation_id = query.reservation_id;
  }

  return api.get<UploadStatusResponse>(
    API_ENDPOINTS.chunkedUpload.status(roomName),
    params,
    { token: authToken },
  );
}

/**
 * Complete a chunked upload
 *
 * @param roomName - The name of the room
 * @param uploadId - The upload ID
 * @param token - Authentication token
 * @returns Completion result
 */
export async function completeChunkedUpload(
  roomName: string,
  request: FileMergeRequest,
  token?: string,
): Promise<FileMergeResponse> {
  const authToken = token || await getValidToken(roomName);
  if (!authToken) {
    throw new Error("Authentication required to complete upload");
  }

  return api.post<FileMergeResponse>(
    API_ENDPOINTS.chunkedUpload.complete(roomName),
    request,
    { token: authToken },
  );
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Calculate SHA256 hash of data
 */
async function calculateSHA256Hash(data: ArrayBuffer | Blob): Promise<string> {
  let buffer: ArrayBuffer;

  if (data instanceof Blob) {
    buffer = await data.arrayBuffer();
  } else {
    buffer = data;
  }

  const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join(
    "",
  );
  return hashHex;
}

/**
 * Format file size for display
 */
export function formatFileSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(1)} ${units[unitIndex]}`;
}

/**
 * Format upload speed for display
 */
export function formatUploadSpeed(bytesPerSecond: number): string {
  return `${formatFileSize(bytesPerSecond)}/s`;
}

/**
 * Format estimated time remaining for display
 */
export function formatTimeRemaining(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  } else if (seconds < 3600) {
    return `${Math.round(seconds / 60)}m`;
  } else {
    return `${Math.round(seconds / 3600)}h`;
  }
}

const chunkedUploadService = {
  uploadFileChunked,
  getChunkedUploadStatus,
  completeChunkedUpload,
  formatFileSize,
  formatUploadSpeed,
  formatTimeRemaining,
};

export default chunkedUploadService;
