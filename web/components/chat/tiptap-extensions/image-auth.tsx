"use client";

import { Image as BaseImage } from "@tiptap/extension-image";
import { ReactNodeViewRenderer, NodeViewWrapper } from "@tiptap/react";
import { useAppStore } from "@/lib/store";
import { getRoomTokenString } from "@/lib/utils/api";
import { useState, useMemo, useEffect } from "react";
import Zoom from "react-medium-image-zoom";
import "react-medium-image-zoom/dist/styles.css";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

function ImageWithAuth({ node, updateAttributes }: any) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const [hasError, setHasError] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [imageKey, setImageKey] = useState(0);

  // 使用 useMemo 确保 URL 计算是响应式的
  const authenticatedSrc = useMemo(() => {
    const originalSrc = node.attrs.src;

    // 只处理相对路径的图片（以 / 开头）
    if (typeof originalSrc === "string" && originalSrc.startsWith("/")) {
      const token = getRoomTokenString(currentRoomId);

      if (!token) {
        console.warn("[ImageAuth] No token for room:", currentRoomId);
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

        console.log("[ImageAuth] Final URL:", finalUrl);
        return finalUrl;
      } catch (e) {
        console.error("[ImageAuth] URL parse error:", e);
        return originalSrc;
      }
    }

    return originalSrc;
  }, [node.attrs.src, currentRoomId]);

  // 监听 src 变化，重置加载状态
  useEffect(() => {
    console.log("[ImageAuth] SRC changed, resetting load state");
    setIsLoading(true);
    setHasError(false);
  }, [authenticatedSrc]);

  const handleError = () => {
    console.error("[ImageAuth] Load failed, attempt:", imageKey + 1);

    if (imageKey < 3) {
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

  const handleLoad = () => {
    console.log("[ImageAuth] Load success:", authenticatedSrc);
    setIsLoading(false);
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
              key={`${imageKey}-${authenticatedSrc}`}
              src={authenticatedSrc}
              alt={node.attrs.alt || "图片"}
              title={node.attrs.title}
              className="max-w-sm max-h-64 object-contain rounded-md border border-border cursor-zoom-in"
              onLoad={handleLoad}
              onError={handleError}
            />
          </Zoom>
        </div>
      )}
    </NodeViewWrapper>
  );
}

export const ImageAuth = BaseImage.extend({
  name: "image",
  priority: 1000,

  addNodeView() {
    return ReactNodeViewRenderer(ImageWithAuth);
  },
});
