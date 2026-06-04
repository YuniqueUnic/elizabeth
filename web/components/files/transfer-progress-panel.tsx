"use client";

import { ArrowDown, ArrowUp, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useAppStore } from "@/lib/store";
import type { TransferItem } from "@/lib/transfer-types";

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "";
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatEta(seconds: number): string {
  if (seconds <= 0 || !Number.isFinite(seconds)) return "";
  if (seconds < 60) return `${Math.ceil(seconds)}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.ceil(seconds % 60)}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function TransferRow({ transfer }: { transfer: TransferItem }) {
  const cancelTransfer = useAppStore((state) => state.cancelTransfer);
  const { progress, direction, status } = transfer;
  const isCancelable = status === "active" || status === "pending";

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 text-xs">
      {direction === "upload"
        ? <ArrowUp className="h-3 w-3 shrink-0 text-blue-500" />
        : <ArrowDown className="h-3 w-3 shrink-0 text-green-500" />}
      <span className="truncate min-w-0 flex-1" title={transfer.fileName}>
        {transfer.fileName}
      </span>
      <span className="shrink-0 tabular-nums text-muted-foreground">
        {progress.percentage.toFixed(0)}%
      </span>
      {status === "active" && (
        <>
          {progress.speed > 0 && (
            <span className="shrink-0 tabular-nums text-muted-foreground">
              {formatSpeed(progress.speed)}
            </span>
          )}
          {progress.estimatedTimeRemaining > 0 && (
            <span className="shrink-0 tabular-nums text-muted-foreground">
              {formatEta(progress.estimatedTimeRemaining)}
            </span>
          )}
        </>
      )}
      {status === "completed" && (
        <span className="shrink-0 text-green-600">✓</span>
      )}
      {status === "error" && (
        <span className="shrink-0 text-destructive" title={transfer.error}>!</span>
      )}
      {status === "cancelled" && (
        <span className="shrink-0 text-muted-foreground">⊘</span>
      )}
      {isCancelable && (
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5 shrink-0"
          onClick={() => cancelTransfer(transfer.id)}
          title="Cancel"
        >
          <X className="h-3 w-3" />
        </Button>
      )}
    </div>
  );
}

export function TransferProgressPanel() {
  const transfers = useAppStore((state) => state.transfers);
  const activeTransfers = Object.values(transfers).filter(
    (t) => t.status === "active" || t.status === "pending",
  );
  const recentTransfers = Object.values(transfers).filter(
    (t) => t.status === "completed" || t.status === "cancelled" || t.status === "error",
  );

  if (activeTransfers.length === 0 && recentTransfers.length === 0) return null;

  return (
    <div className="border-b bg-muted/30">
      {activeTransfers.map((t) => (
        <div key={t.id}>
          <TransferRow transfer={t} />
          <div className="px-3 pb-1">
            <Progress value={t.progress.percentage} className="h-1" />
          </div>
        </div>
      ))}
      {recentTransfers.map((t) => (
        <TransferRow key={t.id} transfer={t} />
      ))}
    </div>
  );
}
