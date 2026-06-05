import { useState, useEffect } from "react";
import Zoom from "react-medium-image-zoom";
import "react-medium-image-zoom/dist/styles.css";
import { Button } from "@/components/ui/button";
import {
  FlipHorizontal,
  FlipVertical,
  RotateCcw,
  RotateCw,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useSecureBlobUrl } from "@/hooks/use-secure-blob-url";

interface ImageViewerProps {
  src: string;
  alt: string;
  roomName?: string;
  className?: string;
}

export function ImageViewer({ src, alt, roomName, className = "" }: ImageViewerProps) {
  const t = useTranslations("room.image");
  const [rotation, setRotation] = useState(0);
  const [flipH, setFlipH] = useState(false);
  const [flipV, setFlipV] = useState(false);
  const [scale, setScale] = useState(1);

  const { blobUrl, loading: isHookLoading } = useSecureBlobUrl(src, roomName);

  const handleRotateRight = () => setRotation((r) => (r + 90) % 360);
  const handleRotateLeft = () => setRotation((r) => (r - 90 + 360) % 360);
  const handleFlipH = () => setFlipH((f) => !f);
  const handleFlipV = () => setFlipV((f) => !f);
  const handleZoomIn = () => setScale((s) => Math.min(s + 0.25, 3));
  const handleZoomOut = () => setScale((s) => Math.max(s - 0.25, 0.5));
  const handleReset = () => {
    setRotation(0);
    setFlipH(false);
    setFlipV(false);
    setScale(1);
  };

  const transformStyle = {
    transform: `rotate(${rotation}deg) scaleX(${flipH ? -1 : 1}) scaleY(${
      flipV ? -1 : 1
    }) scale(${scale})`,
    transition: "transform 0.3s ease",
  };

  const displaySrc = blobUrl || src;

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center justify-center gap-1 px-2 border-b bg-muted/30">
        <Button
          variant="ghost"
          size="sm"
          onClick={handleRotateLeft}
          title={t("rotateLeft")}
        >
          <RotateCcw className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleRotateRight}
          title={t("rotateRight")}
        >
          <RotateCw className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleFlipH}
          title={t("flipH")}
        >
          <FlipHorizontal className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleFlipV}
          title={t("flipV")}
        >
          <FlipVertical className="h-4 w-4" />
        </Button>
        <div className="w-px h-6 bg-border mx-1" />
        <Button
          variant="ghost"
          size="sm"
          onClick={handleZoomOut}
          title={t("zoomOut")}
          disabled={scale <= 0.5}
        >
          <ZoomOut className="h-4 w-4" />
        </Button>
        <span className="text-sm text-muted-foreground min-w-12 text-center">
          {Math.round(scale * 100)}%
        </span>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleZoomIn}
          title={t("zoomIn")}
          disabled={scale >= 3}
        >
          <ZoomIn className="h-4 w-4" />
        </Button>
        <div className="w-px h-6 bg-border mx-1" />
        <Button
          variant="ghost"
          size="sm"
          onClick={handleReset}
          title={t("reset")}
        >
          <RotateCcw className="h-4 w-4" />
        </Button>
      </div>

      {/* Image Container */}
      <div className="flex-1 flex items-center justify-center overflow-auto p-4 bg-muted/10">
        {isHookLoading ? (
          <div className="text-sm text-muted-foreground">{t("loading")}</div>
        ) : (
          <Zoom>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={displaySrc || undefined}
              alt={alt}
              className={className}
              style={transformStyle}
              onError={(e) => {
                console.error("Image load error:", e);
                e.currentTarget.src = "/placeholder.svg";
              }}
            />
          </Zoom>
        )}
      </div>
    </div>
  );
}

