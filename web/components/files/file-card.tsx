"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Trash2, Shield, Lock } from "lucide-react";
import type { FileItem } from "@/lib/types";
import { useAppStore } from "@/lib/store";
import { formatFileSize } from "@/lib/utils/format";
import { defaultStyles, FileIcon } from "react-file-icon";
import { useTranslations } from "next-intl";
import { usePathname } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { getPolicy } from "@/api/policyService";
import { DownloadPolicyDialog } from "./download-policy-dialog";


// Helper function to get file extension
function getFileExtension(filename: string): string {
  const parts = filename.split(".");
  if (parts.length > 1) {
    return parts[parts.length - 1].toLowerCase();
  }
  return "";
}

// Helper function to get file icon props
function getFileIconProps(filename: string) {
  const ext = getFileExtension(filename);

  // Check if we have default styles for this extension
  if (ext && defaultStyles[ext as keyof typeof defaultStyles]) {
    return {
      extension: ext,
      ...defaultStyles[ext as keyof typeof defaultStyles],
    };
  }

  // Fallback to generic document icon
  return {
    extension: ext || "file",
    type: "document" as const,
  };
}

interface FileCardProps {
  file: FileItem;
  onDelete: (fileId: string) => void;
  onClick: (file: FileItem) => void;
  showCheckbox: boolean;
  canDelete: boolean;
}

export function FileCard(
  { file, onDelete, onClick, showCheckbox, canDelete }: FileCardProps,
) {
  const t = useTranslations("room");
  const tp = useTranslations("room.downloadPolicy");
  const { selectedFiles, toggleFileSelection } = useAppStore();
  const isSelected = selectedFiles.has(file.id);
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const pathname = usePathname();
  const roomName = pathname.split("/").filter(Boolean)[0] || currentRoomId;

  const [policyDialogOpen, setPolicyDialogOpen] = useState(false);

  const { data: policy } = useQuery({
    queryKey: ["policy", roomName, file.id],
    queryFn: () => getPolicy(roomName, file.id),
    enabled: !!roomName && !!file.id,
    staleTime: 60000,
  });

  const isProtected = policy && policy.mode !== "off";

  return (
    <>
      <div
        className={`group relative flex items-center gap-3 rounded-lg border p-2 transition-all ${
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
          className="flex min-w-0 flex-1 items-center gap-3 cursor-pointer"
          onClick={() => onClick(file)}
        >
          {/* File Type Icon */}
          <div className="flex h-8 w-8 shrink-0 items-center justify-center">
            <FileIcon {...getFileIconProps(file.name)} />
          </div>

          {/* File Info */}
          <div className="min-w-0 flex-1 overflow-hidden">
            <p
              className="text-sm font-medium break-all line-clamp-3"
              title={file.name}
            >
              {file.name}
            </p>
            <div className="flex items-center gap-2 mt-0.5">
              <p className="text-xs text-muted-foreground">
                {formatFileSize(file.size || 0)}
              </p>
              {isProtected && (
                <Badge variant="secondary" className="px-1 text-[10px] h-4">
                  <Lock className="w-3 h-3 mr-1" />
                  {tp("protectedBadge")}
                </Badge>
              )}
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center opacity-0 transition-opacity group-hover:opacity-100">
          {canDelete && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 shrink-0"
              onClick={(e) => {
                e.stopPropagation();
                setPolicyDialogOpen(true);
              }}
              title={tp("settingsTitle")}
            >
              <Shield className="h-4 w-4 text-muted-foreground" />
            </Button>
          )}

          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 shrink-0"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(file.id);
            }}
            title={t("fileCard.deleteFile")}
            disabled={!canDelete}
          >
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        </div>
      </div>

      {canDelete && policyDialogOpen && (
        <DownloadPolicyDialog
          open={policyDialogOpen}
          onOpenChange={setPolicyDialogOpen}
          roomName={roomName}
          contentId={file.id}
        />
      )}
    </>
  );
}
