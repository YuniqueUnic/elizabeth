"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { FileText, Trash2 } from "lucide-react";
import type { FileItem } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { formatFileSize } from "@/lib/utils/format";

interface FileCardProps {
  file: FileItem;
  onDelete: (fileId: string) => void;
  onClick: (file: FileItem) => void;
  showCheckbox: boolean;
}

export function FileCard(
  { file, onDelete, onClick, showCheckbox }: FileCardProps,
) {
  const { selectedFiles, toggleFileSelection } = useAppStore();
  const isSelected = selectedFiles.has(file.id);

  return (
    <div
      className={`group relative flex items-center gap-3 rounded-lg border p-3 transition-all ${
        isSelected
          ? "border-primary border-2 bg-primary/5 shadow-sm"
          : "border-border bg-card hover:bg-accent/50"
      }`}
    >
      {/* Checkbox */}
      {showCheckbox && (
        <Checkbox
          checked={isSelected}
          onCheckedChange={() => toggleFileSelection(file.id)}
          className="shrink-0"
          onClick={(e) => e.stopPropagation()}
        />
      )}

      <div
        className="flex flex-1 items-center gap-3 cursor-pointer"
        onClick={() => onClick(file)}
      >
        {/* Thumbnail or Icon */}
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md bg-muted">
          {file.thumbnailUrl
            ? (
              <img
                src={file.thumbnailUrl || "/placeholder.svg"}
                alt={file.name}
                className="h-full w-full rounded-md object-cover"
              />
            )
            : <FileText className="h-6 w-6 text-muted-foreground" />}
        </div>

        {/* File Info */}
        <div className="min-w-0 flex-1">
          <p
            className="text-sm font-medium wrap-break-word line-clamp-3"
            title={file.name}
          >
            {file.name}
          </p>
          <p className="text-xs text-muted-foreground">
            {formatFileSize(file.size || 0)}
          </p>
        </div>
      </div>

      {/* Delete Button */}
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 shrink-0 opacity-0 transition-opacity group-hover:opacity-100"
        onClick={(e) => {
          e.stopPropagation();
          onDelete(file.id);
        }}
        title="删除文件"
      >
        <Trash2 className="h-4 w-4 text-destructive" />
      </Button>
    </div>
  );
}
