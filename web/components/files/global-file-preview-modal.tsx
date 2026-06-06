"use client";

/**
 * GlobalFilePreviewModal
 *
 * This component is intentionally mounted at the top-level room layout
 * (RoomClientPage) so it is ALWAYS in the React tree regardless of which
 * mobile tab is active. That makes it possible to open a file preview
 * from a message-bubble click (which sets `previewFileId` in the store)
 * even when the Files tab is not visible / RightSidebar is off-screen.
 *
 * It owns:
 *  - the previewFileId store subscription
 *  - its own files query (staleTime 4 s, same key as RightSidebar so it
 *    reuses the React-Query cache)
 *  - its own delete mutation
 */

import { useEffect, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { usePathname } from "next/navigation";
import { useAppStore } from "@/lib/store";
import { FilePreviewModal } from "@/components/files/file-preview-modal";
import { getFilesList, deleteFile } from "@/api/fileService";
import type { FileItem } from "@/lib/types";
import { useToast } from "@/hooks/use-toast";
import { useTranslations } from "next-intl";

export function GlobalFilePreviewModal() {
  const { toast } = useToast();
  const t = useTranslations("room");
  const queryClient = useQueryClient();

  const pathname = usePathname();
  // Derive roomName from URL path (same logic as RightSidebar)
  const roomName = pathname.split("/").filter(Boolean)[0] || useAppStore.getState().currentRoomId;

  const previewFileId = useAppStore((state) => state.previewFileId);
  const setPreviewFileId = useAppStore((state) => state.setPreviewFileId);

  const [previewFile, setPreviewFile] = useState<FileItem | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  const { data: files = [] } = useQuery({
    queryKey: ["files", roomName],
    queryFn: () => getFilesList(roomName),
    staleTime: 4000,
    enabled: !!roomName,
  });

  // When a file-id arrives from the store (message-bubble click), locate and open it
  useEffect(() => {
    if (!previewFileId || files.length === 0) return;
    const file = files.find((f) => f.id === previewFileId);
    if (file) {
      setPreviewFile(file);
      setPreviewOpen(true);
      setPreviewFileId(null);
    }
  }, [previewFileId, files, setPreviewFileId]);

  const deleteMutation = useMutation({
    mutationFn: (fileId: string) => deleteFile(roomName, fileId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["files", roomName] });
      toast({
        title: t("toast.deleteSuccess"),
        description: t("toast.deleteSuccessDescription"),
      });
    },
    onError: () => {
      toast({
        title: t("toast.deleteFailed"),
        description: t("toast.deleteFailed"),
        variant: "destructive",
      });
    },
  });

  const handleDelete = (fileId: string) => {
    deleteMutation.mutate(fileId);
    setPreviewOpen(false);
    setPreviewFile(null);
  };

  return (
    <FilePreviewModal
      file={previewFile}
      roomName={roomName}
      open={previewOpen}
      onOpenChange={(open) => {
        setPreviewOpen(open);
        if (!open) setPreviewFile(null);
      }}
      onDelete={handleDelete}
    />
  );
}
