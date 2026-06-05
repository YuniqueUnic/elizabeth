"use client";

import { useEffect, useState } from "react";
import { getValidToken } from "@/api/authService";
import { api } from "@/lib/utils/api";

interface SecureBlobUrlResult {
  blobUrl: string | null;
  loading: boolean;
  error: Error | null;
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
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!src) {
      setBlobUrl(null);
      setLoading(false);
      setError(null);
      return;
    }

    // Check if the URL is a relative content path that requires room token authentication
    const isRelativeContent =
      src.startsWith("/") &&
      (src.includes("/contents/") || src.includes("/api/v1/contents/"));

    if (!isRelativeContent || !roomName) {
      setBlobUrl(src);
      setLoading(false);
      setError(null);
      return;
    }

    let isCancelled = false;
    let createdUrl: string | null = null;

    const loadSecureContent = async () => {
      setLoading(true);
      setError(null);

      try {
        // Extract the content ID
        const match = src.match(/contents\/(\d+)/);
        if (!match) {
          if (!isCancelled) {
            setBlobUrl(src);
            setLoading(false);
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
        setBlobUrl(createdUrl);
        setLoading(false);
      } catch (err) {
        console.error(`[useSecureBlobUrl] Failed to load secure content for ${src}:`, err);
        if (!isCancelled) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setLoading(false);
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
  }, [src, roomName]);

  return { blobUrl, loading, error };
}
