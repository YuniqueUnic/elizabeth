"use client";

import { Image as BaseImage } from "@tiptap/extension-image";
import { ReactNodeViewRenderer, NodeViewWrapper } from "@tiptap/react";
import { useAppStore } from "@/lib/store";
import { getRoomTokenString } from "@/lib/utils/api";
import { useState } from "react";
import Zoom from "react-medium-image-zoom";
import "react-medium-image-zoom/dist/styles.css";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

function ImageWithAuth({ node, updateAttributes }: any) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const [hasError, setHasError] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [imageKey, setImageKey] = useState(0); // 用于强制重新加载图片

  // 直接计算带 token 的 URL
  const getAuthenticatedUrl = (originalSrc: string): string => {
    // 只处理相对路径的图片（以 / 开头）
    if (typeof originalSrc === "string" && originalSrc.startsWith("/")) {
      const token = getRoomTokenString(currentRoomId);

      if (!token) {
        console.warn("[ImageAuth] No token found for room:", currentRoomId, "src:", originalSrc);
        return originalSrc;
      }

      try {
        // 确保路径以 /api/v1 开头
        let path = originalSrc;
        if (path.startsWith("/rooms/") && !path.startsWith("/api/v1/")) {
          path = `/api/v1${path}`;
        }

        // 构建完整的 URL
        const url = new URL(path, window.location.origin);
        url.searchParams.set("token", token);
        const finalUrl = `${url.pathname}${url.search}`;

        console.log("[ImageAuth] Authenticated URL:", finalUrl);
        return finalUrl;
      } catch (e) {
        console.error("[ImageAuth] Failed to parse image URL:", e);
        return originalSrc;
      }
    }

    // 外部图片或 data URL，直接使用
    return originalSrc;
  };

  const authenticatedSrc = getAuthenticatedUrl(node.attrs.src);

  // 当图片加载失败时，尝试重新加载
  const handleError = () => {
    console.error("[ImageAuth] Image load failed:", authenticatedSrc);

    // 等待一小段时间后重试（可能是文件还在处理中）
    if (imageKey < 3) {
      console.log("[ImageAuth] Retrying in 1s...");
      setTimeout(() => {
        setImageKey(k => k + 1);
        setIsLoading(true);
        setHasError(false);
      }, 1000);
    } else {
      setHasError(true);
      setIsLoading(false);
    }
  };

  return (
    <NodeViewWrapper className="inline-block leading-none max-w-full relative">
      {isLoading && !hasError && (
        <Skeleton className="w-full h-48 rounded-md" />
      )}

      {hasError ? (
        <div className="flex items-center justify-center w-full h-48 bg-muted rounded-md text-muted-foreground text-sm border border-border">
          图片加载失败
        </div>
      ) : (
        <div className={cn(isLoading ? "hidden" : "block")}>
          <Zoom>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              key={imageKey}
              src={authenticatedSrc}
              alt={node.attrs.alt || "图片"}
              title={node.attrs.title}
              className="max-w-sm max-h-64 object-contain rounded-md border border-border cursor-zoom-in"
              loading="lazy"
              onLoad={() => {
                console.log("[ImageAuth] Image loaded successfully:", authenticatedSrc);
                setIsLoading(false);
              }}
              onError={handleError}
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
