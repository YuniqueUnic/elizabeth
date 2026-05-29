"use client";

import { FileCard } from "./file-card";
import type { FileItem } from "@/lib/types";
import { useTranslations } from "next-intl";

interface FileListViewProps {
  files: FileItem[];
  onDelete: (fileId: string) => void;
  onFileClick: (file: FileItem) => void;
  showCheckboxes: boolean;
}

export function FileListView(
  { files, onDelete, onFileClick, showCheckboxes }: FileListViewProps,
) {
  const t = useTranslations("room");

  if (files.length === 0) {
    return (
      <div className="flex h-32 items-center justify-center rounded-lg border border-dashed">
        <p className="text-sm text-muted-foreground">{t("fileListView.empty")}</p>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {files.map((file) => (
        <FileCard
          key={file.id}
          file={file}
          onDelete={onDelete}
          onClick={onFileClick}
          showCheckbox={showCheckboxes}
        />
      ))}
    </div>
  );
}
