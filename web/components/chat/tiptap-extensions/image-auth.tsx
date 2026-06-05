"use client";

import { Image as BaseImage } from "@tiptap/extension-image";
import { ReactNodeViewRenderer, NodeViewWrapper } from "@tiptap/react";
import { useAppStore } from "@/lib/store";
import { useState, useEffect } from "react";
import { useTranslations } from "next-intl";
import Zoom from "react-medium-image-zoom";
import "react-medium-image-zoom/dist/styles.css";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { useSecureBlobUrl } from "@/hooks/use-secure-blob-url";

function ImageWithAuth({ node }: any) {
  const t = useTranslations("room.image");
  const currentRoomId = useAppStore((state) => state.currentRoomId);
  const [imageKey, setImageKey] = useState(0);
  const [hasError, setHasError] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  const originalSrc = node.attrs.src;
  const srcWithRetry = typeof originalSrc === "string" && originalSrc.startsWith("/")
    ? `${originalSrc}${originalSrc.includes("?") ? "&" : "?"}_retry=${imageKey}`
    : originalSrc;

  const { blobUrl, loading: isHookLoading, error: hookError } = useSecureBlobUrl(
    srcWithRetry,
    currentRoomId,
  );

  // Sync hook states to local states
  useEffect(() => {
    if (isHookLoading) {
      setIsLoading(true);
      setHasError(false);
    }
  }, [isHookLoading]);

  useEffect(() => {
    if (hookError) {
      handleError();
    }
  }, [hookError]);

  const handleError = () => {
    console.error("[ImageAuth] Load failed, attempt:", imageKey + 1);

    if (imageKey < 3) {
      setTimeout(() => {
        setImageKey((k) => k + 1);
        setIsLoading(true);
        setHasError(false);
      }, 1000);
    } else {
      setHasError(true);
      setIsLoading(false);
    }
  };

  const handleLoad = () => {
    console.log("[ImageAuth] Load success:", blobUrl);
    setIsLoading(false);
  };

  const displaySrc = blobUrl || originalSrc;

  return (
    <NodeViewWrapper className="inline-block leading-none max-w-full relative">
      {(isLoading || isHookLoading) && !hasError && (
        <Skeleton className="w-full h-48 rounded-md" />
      )}

      {hasError ? (
        <div className="flex items-center justify-center w-full h-48 bg-muted rounded-md text-muted-foreground text-sm border border-border">
          {t("loadFailed")}
        </div>
      ) : (
        <div className={cn(isLoading || isHookLoading ? "hidden" : "block")}>
          <Zoom>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              key={`${imageKey}-${displaySrc}`}
              src={displaySrc || undefined}
              alt={node.attrs.alt || t("defaultAlt")}
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

