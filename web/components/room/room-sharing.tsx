"use client";

import { Button } from "@/components/ui/button";
import { Download, LinkIcon } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { getQRCodeImage, getShareLink } from "@/api/shareService";
import { useEffect, useState } from "react";
import { useTheme } from "@/lib/hooks/use-theme";

interface RoomSharingProps {
  roomId: string;
}

export function RoomSharing({ roomId }: RoomSharingProps) {
  const [copied, setCopied] = useState(false);
  const { theme } = useTheme();
  const [currentTheme, setCurrentTheme] = useState<"light" | "dark">("light");

  // 监听主题变化，确保二维码主题同步
  useEffect(() => {
    const root = window.document.documentElement;
    const isDark = root.classList.contains("dark");
    setCurrentTheme(isDark ? "dark" : "light");
  }, [theme]);

  const { data: qrCodeUrl } = useQuery({
    queryKey: ["qrcode", roomId, currentTheme],
    queryFn: () => getQRCodeImage(roomId, { theme: currentTheme }),
    enabled: !!roomId,
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
      {qrCodeUrl
        ? (
          <div className="flex justify-center rounded-lg border border-border bg-background p-4">
            <img
              src={qrCodeUrl}
              alt="Room QR Code"
              className="h-40 w-40"
              onError={(e) => {
                console.error("Failed to load QR code:", e);
                e.currentTarget.src = "/placeholder.svg";
              }}
            />
          </div>
        )
        : (
          <div className="flex justify-center rounded-lg border border-border bg-muted p-4">
            <p className="text-sm text-muted-foreground">正在生成二维码...</p>
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
