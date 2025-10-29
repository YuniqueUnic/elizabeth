/**
 * Share Service
 *
 * This service handles sharing operations including:
 * - Generating authenticated share links
 * - Generating QR codes
 * - Managing share permissions
 * - Creating share tokens
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import { canShareRoom, getUserPermissions } from "./permissionService";
import { accessRoom } from "./roomAccessService";
import QRCode from "qrcode";

// ============================================================================
// Share Service Types
// ============================================================================

export interface ShareOptions {
  includePassword?: boolean;
  expiresAt?: string;
  maxAccesses?: number;
  customMessage?: string;
}

export interface ShareLinkData {
  url: string;
  token?: string;
  expiresAt?: string;
  maxAccesses?: number;
  qrCode?: string;
  permissions: string[];
}

export interface ShareTokenResponse {
  share_token: string;
  expires_at: string;
  max_uses?: number;
  permissions: string[];
  room_name: string;
}

// ============================================================================
// Share Functions
// ============================================================================

/**
 * Get share link for a room
 * Checks permissions and returns appropriate shareable URL
 *
 * @param roomName - The name of the room
 * @param token - Optional authentication token
 * @returns The share URL
 */
export async function getShareLink(
  roomName: string,
  token?: string,
): Promise<string> {
  // Check if user has share permission
  if (!(await canShareRoom(roomName, token))) {
    throw new Error("您没有分享此房间的权限");
  }

  if (typeof window === "undefined") {
    return `https://elizabeth.app/room/${roomName}`;
  }

  // Check if room is shareable (has SHARE permission)
  const { checkRoomAvailability } = await import("./roomAccessService");
  const availability = await checkRoomAvailability(roomName);

  if (availability.isShareable) {
    return `${window.location.origin}/${roomName}`;
  } else {
    // For non-shareable rooms, we would need to generate a UUID-based link
    // This would be handled by the backend API
    throw new Error("此房间不允许直接分享");
  }
}

/**
 * Create a share token for a room
 * Generates a temporary token that can be used to access the room
 *
 * @param roomName - The name of the room
 * @param options - Share options
 * @param token - Optional authentication token
 * @returns Share token response
 */
export async function createShareToken(
  roomName: string,
  options: ShareOptions = {},
  token?: string,
): Promise<ShareTokenResponse> {
  const authToken = token || await getValidToken(roomName);

  if (!authToken) {
    throw new Error("需要登录才能创建分享令牌");
  }

  // Check permissions
  const userPerms = await getUserPermissions(roomName, authToken);
  if (!userPerms.canShare) {
    throw new Error("您没有分享此房间的权限");
  }

  // Create share token via backend API
  const response = await api.post<ShareTokenResponse>(
    `${API_ENDPOINTS.rooms.base(roomName)}/share`,
    {
      expires_at: options.expiresAt,
      max_uses: options.maxAccesses,
      include_password: options.includePassword,
      custom_message: options.customMessage,
    },
    { token: authToken },
  );

  return response;
}

/**
 * Get comprehensive share data for a room
 * Includes share link, QR code, and permissions
 *
 * @param roomName - The name of the room
 * @param options - Share options
 * @param token - Optional authentication token
 * @returns Complete share data
 */
export async function getShareData(
  roomName: string,
  options: ShareOptions = {},
  token?: string,
): Promise<ShareLinkData> {
  const shareLink = await getShareLink(roomName, token);
  const userPerms = await getUserPermissions(roomName, token);

  // Generate QR code
  const qrCode = await getQRCodeImage(roomName, {
    width: 300,
    margin: 2,
    errorCorrectionLevel: "M",
  });

  return {
    url: shareLink,
    qrCode,
    permissions: userPerms.permissions,
  };
}

/**
 * Generate QR code image data URL for a room
 *
 * @param roomName - The name of the room
 * @param options - QR code generation options
 * @returns Promise that resolves to a data URL of the QR code image
 */
export async function getQRCodeImage(
  roomName: string,
  options?: {
    width?: number;
    margin?: number;
    errorCorrectionLevel?: "L" | "M" | "Q" | "H";
    includeLogo?: boolean;
    customData?: Record<string, any>;
    theme?: "light" | "dark" | "system";
  },
): Promise<string> {
  const shareLink = await getShareLink(roomName);

  let qrContent = shareLink;

  // Add custom data to QR code if provided
  if (options?.customData) {
    const customParams = new URLSearchParams(options.customData);
    qrContent = `${shareLink}?${customParams.toString()}`;
  }

  // Get theme-aware colors
  const getThemeColors = () => {
    const theme = options?.theme ||
      (typeof window !== "undefined" &&
       (window.document.documentElement.classList.contains("dark") ? "dark" : "light")) ||
      "light";

    if (theme === "dark") {
      return {
        dark: "#FFFFFF", // White QR code on dark background
        light: "#1e293b", // Dark blue background
      };
    } else {
      return {
        dark: "#1e293b", // Dark blue QR code on light background
        light: "#FFFFFF", // White background
      };
    }
  };

  const colors = getThemeColors();

  // Use qrcode library to generate QR code
  return await QRCode.toDataURL(qrContent, {
    width: options?.width || 300,
    margin: options?.margin || 2,
    errorCorrectionLevel: options?.errorCorrectionLevel || "M",
    color: colors,
  });
}

/**
 * Download QR code as an image file
 *
 * @param roomName - The name of the room
 * @param filename - Optional filename for the download
 * @param options - QR code generation options
 */
export async function downloadQRCode(
  roomName: string,
  filename?: string,
  options?: {
    width?: number;
    margin?: number;
    errorCorrectionLevel?: "L" | "M" | "Q" | "H";
  },
): Promise<void> {
  const qrImageUrl = await getQRCodeImage(roomName, options);

  // Create a download link for the data URL
  const link = document.createElement("a");
  link.href = qrImageUrl;
  link.download = filename || `${roomName}-qr.png`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/**
 * Copy share link to clipboard
 *
 * @param roomName - The name of the room
 * @param token - Optional authentication token
 * @returns Promise that resolves when the link is copied
 */
export async function copyShareLink(
  roomName: string,
  token?: string,
): Promise<void> {
  const shareLink = await getShareLink(roomName, token);

  if (navigator.clipboard && navigator.clipboard.writeText) {
    await navigator.clipboard.writeText(shareLink);
  } else {
    // Fallback for older browsers
    const textArea = document.createElement("textarea");
    textArea.value = shareLink;
    textArea.style.position = "fixed";
    textArea.style.left = "-999999px";
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();

    try {
      document.execCommand("copy");
    } finally {
      document.body.removeChild(textArea);
    }
  }
}

/**
 * Validate and access a shared room
 * Used when someone clicks a shared link
 *
 * @param roomName - The name of the room
 * @param shareToken - Optional share token from URL
 * @param password - Optional password from URL params
 * @returns Room access result
 */
export async function accessSharedRoom(
  roomName: string,
  shareToken?: string,
  password?: string,
): Promise<{
  success: boolean;
  roomDetails?: any;
  error?: string;
  requiresPassword?: boolean;
}> {
  try {
    // If we have a share token, try to use it
    if (shareToken) {
      const response = await api.post(
        `${API_ENDPOINTS.rooms.base(roomName)}/share/access`,
        { share_token: shareToken, password },
        { skipTokenInjection: true },
      );

      return {
        success: true,
        roomDetails: response,
      };
    }

    // Fall back to normal room access
    const accessResult = await accessRoom(roomName, {
      password,
      skipCache: true,
    });

    if (accessResult.success) {
      return {
        success: true,
        roomDetails: accessResult.roomDetails,
      };
    } else {
      return {
        success: false,
        error: accessResult.error,
        requiresPassword: accessResult.requiresPassword,
      };
    }
  } catch (error: any) {
    return {
      success: false,
      error: error.message || "无法访问房间",
    };
  }
}

/**
 * Share room via Web Share API (if available)
 *
 * @param roomName - The name of the room
 * @param title - Optional title for the share
 * @param text - Optional text for the share
 * @param token - Optional authentication token
 * @returns Promise that resolves when share is attempted
 */
export async function shareViaWebShare(
  roomName: string,
  title?: string,
  text?: string,
  token?: string,
): Promise<void> {
  if (!navigator.share) {
    throw new Error("您的浏览器不支持分享功能");
  }

  const shareLink = await getShareLink(roomName, token);
  const shareData = {
    title: title || `Elizabeth 房间：${roomName}`,
    text: text || `加入我在 Elizabeth 创建的房间：${roomName}`,
    url: shareLink,
  };

  try {
    await navigator.share(shareData);
  } catch (error) {
    if ((error as any).name !== "AbortError") {
      throw error;
    }
    // User cancelled the share - this is not an error
  }
}

/**
 * Check if Web Share API is available
 */
export function isWebShareAvailable(): boolean {
  return typeof navigator !== "undefined" && "share" in navigator;
}

export default {
  getShareLink,
  createShareToken,
  getShareData,
  getQRCodeImage,
  downloadQRCode,
  copyShareLink,
  accessSharedRoom,
  shareViaWebShare,
  isWebShareAvailable,
};
