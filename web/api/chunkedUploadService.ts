/**
 * Chunked Upload Service
 *
 * This service handles large file uploads by splitting them into chunks
 * and uploading them individually with progress tracking.
 */

import { API_ENDPOINTS } from "../lib/config";
import { api, buildURL } from "../lib/utils/api";
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
      const url = buildURL(API_ENDPOINTS.chunkedUpload.upload(roomName));
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

function rightRotate(value: number, amount: number): number {
  return (value >>> amount) | (value << (32 - amount));
}

function sha256Fallback(buffer: Uint8Array): string {
  const h = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
  ];

  const k = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ];

  const asciiBitLength = buffer.length * 8;
  const words: number[] = [];

  for (let i = 0; i < buffer.length; i++) {
    words[i >> 2] |= (buffer[i] & 0xff) << (24 - (i % 4) * 8);
  }

  words[asciiBitLength >> 5] |= 0x80 << (24 - (asciiBitLength % 32));
  words[(((asciiBitLength + 64) >> 9) << 4) + 15] = asciiBitLength;

  let h0 = h[0], h1 = h[1], h2 = h[2], h3 = h[3], h4 = h[4], h5 = h[5], h6 = h[6], h7 = h[7];

  for (let i = 0; i < words.length; i += 16) {
    const w: number[] = [];
    for (let j = 0; j < 64; j++) {
      if (j < 16) {
        w[j] = words[i + j] | 0;
      } else {
        const s0 = rightRotate(w[j - 15], 7) ^ rightRotate(w[j - 15], 18) ^ (w[j - 15] >>> 3);
        const s1 = rightRotate(w[j - 2], 17) ^ rightRotate(w[j - 2], 19) ^ (w[j - 2] >>> 10);
        w[j] = (w[j - 16] + s0 + w[j - 7] + s1) | 0;
      }

      const ch = (h4 & h5) ^ (~h4 & h6);
      const maj = (h0 & h1) ^ (h0 & h2) ^ (h1 & h2);
      const s0 = rightRotate(h0, 2) ^ rightRotate(h0, 13) ^ rightRotate(h0, 22);
      const s1 = rightRotate(h4, 6) ^ rightRotate(h4, 11) ^ rightRotate(h4, 25);

      const temp1 = (h7 + s1 + ch + k[j] + w[j]) | 0;
      const temp2 = (s0 + maj) | 0;

      h7 = h6;
      h6 = h5;
      h5 = h4;
      h4 = (h3 + temp1) | 0;
      h3 = h2;
      h2 = h1;
      h1 = h0;
      h0 = (temp1 + temp2) | 0;
    }

    h0 = (h0 + h[0]) | 0;
    h1 = (h1 + h[1]) | 0;
    h2 = (h2 + h[2]) | 0;
    h3 = (h3 + h[3]) | 0;
    h4 = (h4 + h[4]) | 0;
    h5 = (h5 + h[5]) | 0;
    h6 = (h6 + h[6]) | 0;
    h7 = (h7 + h[7]) | 0;

    h[0] = h0;
    h[1] = h1;
    h[2] = h2;
    h[3] = h3;
    h[4] = h4;
    h[5] = h5;
    h[6] = h6;
    h[7] = h7;
  }

  return [h0, h1, h2, h3, h4, h5, h6, h7].map(val => {
    return (val >>> 0).toString(16).padStart(8, '0');
  }).join('');
}

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

  if (typeof window === "undefined" || typeof crypto === "undefined" || !crypto.subtle) {
    console.warn("crypto.subtle is not available, using pure JS fallback");
    return sha256Fallback(new Uint8Array(buffer));
  }

  try {
    const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join(
      "",
    );
    return hashHex;
  } catch (err) {
    console.error("crypto.subtle.digest failed, using pure JS fallback", err);
    return sha256Fallback(new Uint8Array(buffer));
  }
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
