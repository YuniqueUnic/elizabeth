export type TransferDirection = "upload" | "download";
export type TransferStatus = "pending" | "active" | "completed" | "cancelled" | "error";

export interface TransferProgress {
  bytesTransferred: number;
  totalBytes: number;
  percentage: number;
  speed: number; // bytes/sec
  estimatedTimeRemaining: number; // seconds
}

export interface TransferItem {
  id: string;
  fileName: string;
  fileSize: number;
  direction: TransferDirection;
  status: TransferStatus;
  progress: TransferProgress;
  reservationId?: string;
  startedAt: number;
  error?: string;
  abortController: AbortController;
}
