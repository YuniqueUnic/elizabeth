"use client";

import { Button } from "@/components/ui/button";
import { Download, LinkIcon } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { getQRCodeImage, getShareLink } from "@/api/shareService";
import { useState } from "react";

interface RoomSharingProps {
  roomId: string;
}

export function RoomSharing({ roomId }: RoomSharingProps) {
  const [copied, setCopied] = useState(false);

  const { data: qrCodeUrl } = useQuery({
    queryKey: ["qrcode", roomId],
    queryFn: () => getQRCodeImage(roomId),
  });

  const { data: shareLink } = useQuery({
    queryKey: ["sharelink", roomId],
    queryFn: () => getShareLink(roomId),
  });

  const handleCopyLink = async () => {
    if (shareLink) {
      await navigator.clipboard.writeText(shareLink);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleDownloadQR = () => {
    if (qrCodeUrl) {
      const link = document.createElement("a");
      link.href = qrCodeUrl;
      link.download = `elizabeth-room-${roomId}-qr.png`;
      link.click();
    }
  };

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold">分享房间</h3>

      {/* QR Code */}
      {qrCodeUrl && (
        <div className="flex justify-center rounded-lg border bg-white p-4">
          <img
            src={qrCodeUrl || "/placeholder.svg"}
            alt="Room QR Code"
            className="h-40 w-40"
          />
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2">
        <Button
          variant="outline"
          className="flex-1 bg-transparent"
          onClick={handleCopyLink}
        >
          <LinkIcon className="mr-2 h-4 w-4" />
          {copied ? "已复制" : "获取链接"}
        </Button>
        <Button
          variant="outline"
          className="flex-1 bg-transparent"
          onClick={handleDownloadQR}
        >
          <Download className="mr-2 h-4 w-4" />
          下载
        </Button>
      </div>

      {shareLink && (
        <p className="text-xs text-muted-foreground break-all">{shareLink}</p>
      )}
    </div>
  );
}
