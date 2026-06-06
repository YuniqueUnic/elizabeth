"use client";

import { useCallback, useRef, useState, useEffect } from "react";
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
}

export function ImageViewer({ src, alt, roomName }: ImageViewerProps) {
  const t = useTranslations("room.image");
  const [rotation, setRotation] = useState(0);
  const [flipH, setFlipH] = useState(false);
  const [flipV, setFlipV] = useState(false);
  const [scale, setScale] = useState(1);
  // Pan offset (translate in CSS pixels)
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  // Drag state stored in refs to avoid re-renders mid-drag
  const isDragging = useRef(false);
  const dragStart = useRef({ x: 0, y: 0 });
  const offsetAtDragStart = useRef({ x: 0, y: 0 });

  const { blobUrl, loading } = useSecureBlobUrl(src, roomName);
  const displaySrc = blobUrl || src;

  const clampScale = (v: number) => Math.min(Math.max(v, 0.25), 5);

  // ── Wheel zoom ──────────────────────────────────────────────────────────────
  const handleWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.12 : -0.12;
    setScale((s) => {
      const next = clampScale(s + delta);
      // Reset pan when zooming back to fit
      if (next <= 1) setOffset({ x: 0, y: 0 });
      return next;
    });
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener("wheel", handleWheel, { passive: false });
    return () => el.removeEventListener("wheel", handleWheel);
  }, [handleWheel]);

  // ── Drag-to-pan (only when zoomed in) ──────────────────────────────────────
  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (scale <= 1) return;           // no pan needed at fit-to-view
    e.currentTarget.setPointerCapture(e.pointerId);
    isDragging.current = true;
    dragStart.current = { x: e.clientX, y: e.clientY };
    offsetAtDragStart.current = { ...offset };
  }, [scale, offset]);

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!isDragging.current) return;
    const dx = e.clientX - dragStart.current.x;
    const dy = e.clientY - dragStart.current.y;
    setOffset({
      x: offsetAtDragStart.current.x + dx,
      y: offsetAtDragStart.current.y + dy,
    });
  }, []);

  const handlePointerUp = useCallback(() => {
    isDragging.current = false;
  }, []);

  const handleReset = () => {
    setRotation(0);
    setFlipH(false);
    setFlipV(false);
    setScale(1);
    setOffset({ x: 0, y: 0 });
  };

  // Reset offset when zooming back to 1
  useEffect(() => {
    if (scale <= 1) setOffset({ x: 0, y: 0 });
  }, [scale]);

  const transform = [
    `translate(${offset.x}px, ${offset.y}px)`,
    `rotate(${rotation}deg)`,
    `scaleX(${flipH ? -1 : 1})`,
    `scaleY(${flipV ? -1 : 1})`,
    `scale(${scale})`,
  ].join(" ");

  return (
    <div className="flex flex-col h-full min-h-0">
      {/* Viewer-level toolbar — only image-specific controls */}
      <div className="flex items-center justify-center gap-0.5 px-2 py-1 border-b bg-muted/20 shrink-0">
        <Button variant="ghost" size="sm" onClick={() => setRotation((r) => (r - 90 + 360) % 360)} title={t("rotateLeft")}>
          <RotateCcw className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" onClick={() => setRotation((r) => (r + 90) % 360)} title={t("rotateRight")}>
          <RotateCw className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" onClick={() => setFlipH((f) => !f)} title={t("flipH")}>
          <FlipHorizontal className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" onClick={() => setFlipV((f) => !f)} title={t("flipV")}>
          <FlipVertical className="h-4 w-4" />
        </Button>

        <div className="w-px h-5 bg-border mx-1" />

        <Button variant="ghost" size="sm" onClick={() => setScale((s) => clampScale(s - 0.25))} disabled={scale <= 0.25} title={t("zoomOut")}>
          <ZoomOut className="h-4 w-4" />
        </Button>
        <span className="text-xs text-muted-foreground min-w-[3.5rem] text-center tabular-nums">
          {Math.round(scale * 100)}%
        </span>
        <Button variant="ghost" size="sm" onClick={() => setScale((s) => clampScale(s + 0.25))} disabled={scale >= 5} title={t("zoomIn")}>
          <ZoomIn className="h-4 w-4" />
        </Button>

        <div className="w-px h-5 bg-border mx-1" />

        <Button variant="ghost" size="sm" onClick={handleReset} title={t("reset")}>
          <RotateCcw className="h-3 w-3 opacity-60" />
        </Button>
      </div>

      {/* Image canvas — overflow:hidden prevents any scrollbar leak */}
      <div
        ref={containerRef}
        className="flex-1 min-h-0 overflow-hidden flex items-center justify-center bg-muted/10 select-none"
        style={{ cursor: scale > 1 ? "grab" : "zoom-in" }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      >
        {loading
          ? (
            <span className="text-sm text-muted-foreground animate-pulse">{t("loading")}</span>
          )
          : (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={displaySrc || undefined}
              alt={alt}
              draggable={false}
              style={{
                transform,
                // Only transition non-drag moves (toolbar buttons & reset)
                // During active drag we skip transition for immediate feel
                transition: isDragging.current ? "none" : "transform 0.2s ease",
                maxWidth: "100%",
                maxHeight: "100%",
                objectFit: "contain",
                transformOrigin: "center center",
                pointerEvents: "none",        // let container capture all pointer events
              }}
              onError={(e) => {
                e.currentTarget.src = "/placeholder.svg";
              }}
            />
          )}
      </div>
    </div>
  );
}
