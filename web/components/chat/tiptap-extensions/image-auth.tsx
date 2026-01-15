"use client";

import { Image as BaseImage } from "@tiptap/extension-image";
import { ReactNodeViewRenderer, NodeViewWrapper } from "@tiptap/react";
import { useAppStore } from "@/lib/store";
import { API_BASE_URL } from "@/lib/config";
import { getRoomTokenString } from "@/lib/utils/api";
import { useEffect, useState } from "react";
import Zoom from "react-medium-image-zoom";
import "react-medium-image-zoom/dist/styles.css";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

function ImageWithAuth({ node, updateAttributes }: any) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const [src, setSrc] = useState(node.attrs.src);
  const [isLoaded, setIsLoaded] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    const originalSrc = node.attrs.src;
    if (typeof originalSrc === "string" && originalSrc.startsWith("/")) {
      const token = getRoomTokenString(currentRoomId);
      const hasApiPrefix = originalSrc.startsWith(`${API_BASE_URL}/`);
      const roomPath = originalSrc.startsWith("/rooms/")
        ? `${API_BASE_URL}${originalSrc}`
        : originalSrc;
      const finalPath = hasApiPrefix ? originalSrc : roomPath;

      if (token) {
        try {
          const url = new URL(finalPath, window.location.origin);
          url.searchParams.set("token", token);
          setSrc(`${url.pathname}${url.search}${url.hash}`);
        } catch (e) {
          console.error("Failed to parse image URL:", e);
          setSrc(finalPath);
        }
      } else {
        setSrc(finalPath);
      }
    } else {
      setSrc(originalSrc);
    }
  }, [node.attrs.src, currentRoomId]);

  return (
    <NodeViewWrapper className="inline-block leading-none max-w-full relative">
      {!isLoaded && !hasError && (
        <Skeleton className="w-full h-48 rounded-md" />
      )}

      {hasError ? (
        <div className="flex items-center justify-center w-full h-48 bg-muted rounded-md text-muted-foreground text-sm border border-border">
          Wait... Image failed to load
        </div>
      ) : (
        <div className={cn(isLoaded ? "block" : "hidden")}>
          <Zoom>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={src}
              alt={node.attrs.alt}
              title={node.attrs.title}
              className="max-w-full rounded-md border border-border"
              loading="lazy"
              onLoad={() => setIsLoaded(true)}
              onError={() => setHasError(true)}
            />
          </Zoom>
        </div>
      )}
    </NodeViewWrapper>
  );
}

export const ImageAuth = BaseImage.extend({
  addNodeView() {
    return ReactNodeViewRenderer(ImageWithAuth);
  },
});
