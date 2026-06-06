"use client";

import { useEffect, useState } from "react";
import { getValidToken } from "@/api/authService";
import { api } from "@/lib/utils/api";

interface SecureBlobUrlResult {
  blobUrl: string | null;
  resolvedSrc: string | null;
  loading: boolean;
  error: Error | null;
  requiresAuth: boolean;
}

/**
 * A hook that securely loads an authenticated file/content path by fetching it
 * with the room's JWT token in the Authorization header, converting it to a local
 * blob URL. This prevents JWT tokens from being exposed in URL query parameters in the DOM
 * (which can be copied via right click, or leaked to third parties via Referer headers).
 *
 * @param src The source path or URL (e.g. /contents/36 or /api/v1/contents/36)
 * @param roomName The room name for which the token should be retrieved
 */
export function useSecureBlobUrl(
  src: string | undefined,
  roomName: string | undefined,
): SecureBlobUrlResult {
  const [blobUrlState, setBlobUrlState] = useState<{
    src: string;
    url: string;
  } | null>(null);
  const [errorState, setErrorState] = useState<{
    src: string;
    error: Error;
  } | null>(null);

  const requiresAuth = Boolean(
    src &&
      src.startsWith("/") &&
      (src.includes("/contents/") || src.includes("/api/v1/contents/")),
  );
  const canAuthenticate = requiresAuth && Boolean(roomName);
  const blobUrl = blobUrlState && blobUrlState.src === src ? blobUrlState.url : null;
  const error = errorState && errorState.src === src ? errorState.error : null;
  const resolvedSrc = requiresAuth ? blobUrl : src ?? null;
  const loading = requiresAuth && !blobUrl && !error;

  useEffect(() => {
    if (!src) {
      setBlobUrlState(null);
      setErrorState(null);
      return;
    }

    if (!requiresAuth) {
      setBlobUrlState(null);
      setErrorState(null);
      return;
    }

    if (!canAuthenticate || !roomName) {
      setBlobUrlState(null);
      setErrorState(null);
      return;
    }

    let isCancelled = false;
    let createdUrl: string | null = null;

    const loadSecureContent = async () => {
      setBlobUrlState(null);
      setErrorState(null);

      try {
        // Extract the content ID
        const match = src.match(/contents\/(\d+)/);
        if (!match) {
          if (!isCancelled) {
            setErrorState({
              src,
              error: new Error(`Could not extract content id from secure src: ${src}`),
            });
          }
          return;
        }

        const contentId = match[1];
        const path = `/api/v1/contents/${contentId}`;

        // Get a valid room token (automatically handles refreshes)
        const token = await getValidToken(roomName);
        if (!token) {
          throw new Error("No valid token available for room");
        }

        // Fetch the file securely using Authorization header
        const responseBlob = await api.get<Blob>(
          path,
          undefined,
          {
            token,
            responseType: "blob",
          },
        );

        if (isCancelled) {
          return;
        }

        createdUrl = URL.createObjectURL(responseBlob);
        setBlobUrlState({ src, url: createdUrl });
      } catch (err) {
        console.error(`[useSecureBlobUrl] Failed to load secure content for ${src}:`, err);
        if (!isCancelled) {
          setErrorState({
            src,
            error: err instanceof Error ? err : new Error(String(err)),
          });
        }
      }
    };

    loadSecureContent();

    return () => {
      isCancelled = true;
      if (createdUrl) {
        URL.revokeObjectURL(createdUrl);
      }
    };
  }, [canAuthenticate, requiresAuth, roomName, src]);

  return { blobUrl, resolvedSrc, loading, error, requiresAuth };
}
