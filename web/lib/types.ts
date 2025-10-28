// Core type definitions for Elizabeth platform

// ============================================================================
// Backend API Types (aligned with Rust backend)
// ============================================================================

/**
 * Content Type enumeration (matches backend ContentType enum)
 */
export enum ContentType {
  Text = 0,
  Image = 1,
  File = 2,
  Url = 3,
}

/**
 * Backend ContentType response format (tagged enum)
 */
export type BackendContentType =
  | { type: "text" }
  | { type: "image" }
  | { type: "file" }
  | { type: "url" };

/**
 * Convert backend ContentType to frontend enum
 */
export function parseContentType(
  backendType: BackendContentType | number,
): ContentType {
  if (typeof backendType === "number") {
    return backendType as ContentType;
  }

  const typeMap: Record<string, ContentType> = {
    text: ContentType.Text,
    image: ContentType.Image,
    file: ContentType.File,
    url: ContentType.Url,
  };

  return typeMap[backendType.type] ?? ContentType.File;
}

/**
 * Room permission bits (backend uses bitflags)
 * - READ = 1
 * - EDIT = 2
 * - SHARE = 4
 * - DELETE = 8
 */
export type RoomPermission = "read" | "edit" | "share" | "delete";

/**
 * Backend Room response
 */
export interface BackendRoom {
  name: string;
  permission: number; // Bitflags: 1=read, 2=edit, 4=share, 8=delete
  max_size: number;
  current_size: number;
  times_entered: number;
  max_times_entered: number;
  created_at: string;
  expires_at: string | null;
}

/**
 * Backend RoomContent response
 */
export interface BackendRoomContent {
  id: number;
  content_type: BackendContentType | number;
  file_name?: string;
  url?: string | null;
  size?: number;
  mime_type?: string;
  text?: string;
  created_at: string;
  updated_at: string;
}

/**
 * Backend Token response
 */
export interface BackendTokenResponse {
  token: string;
  expires_at: string;
  refresh_token?: string;
}

/**
 * Backend Token validation response
 */
export interface BackendTokenValidation {
  valid: boolean;
  room_name: string;
  permission: number;
  expires_at: string;
}

// ============================================================================
// Frontend Types
// ============================================================================

export interface RoomSettings {
  expiresAt: string | null;
  passwordProtected: boolean;
  password?: string;
  maxViews: number;
}

export interface RoomDetails {
  id: string;
  name: string;
  currentSize: number; // in bytes
  maxSize: number; // in bytes
  timesEntered: number;
  maxTimesEntered: number;
  settings: RoomSettings;
  permissions: RoomPermission[];
  createdAt: string;
}

export interface Message {
  id: string;
  content: string;
  timestamp: string;
  fileName?: string;
}

export interface FileItem {
  id: string;
  name: string;
  thumbnailUrl: string | null;
  size?: number; // in bytes
  type?: "image" | "video" | "pdf" | "link" | "document";
  url?: string;
  mimeType?: string;
  createdAt?: string;
}

/**
 * Token information stored in localStorage
 */
export interface TokenInfo {
  token: string;
  expiresAt: string;
  refreshToken?: string;
}

/**
 * Token storage format (maps room name to token info)
 */
export type TokenStorage = Record<string, TokenInfo>;

export type Theme = "dark" | "light" | "system";

// ============================================================================
// Permission Utilities
// ============================================================================

/**
 * Convert backend permission bits to frontend permission strings
 */
export function parsePermissions(bits: number): RoomPermission[] {
  const perms: RoomPermission[] = [];
  if (bits & 1) perms.push("read");
  if (bits & 2) perms.push("edit");
  if (bits & 4) perms.push("share");
  if (bits & 8) perms.push("delete");
  return perms;
}

/**
 * Convert frontend permission strings to backend permission bits
 */
export function encodePermissions(perms: RoomPermission[]): number {
  let bits = 0;
  if (perms.includes("read")) bits |= 1;
  if (perms.includes("edit")) bits |= 2;
  if (perms.includes("share")) bits |= 4;
  if (perms.includes("delete")) bits |= 8;
  return bits;
}

/**
 * Convert backend Room to frontend RoomDetails
 */
export function backendRoomToRoomDetails(room: BackendRoom): RoomDetails {
  return {
    id: room.name,
    name: room.name,
    currentSize: room.current_size,
    maxSize: room.max_size,
    timesEntered: room.times_entered,
    maxTimesEntered: room.max_times_entered,
    settings: {
      expiresAt: room.expires_at,
      passwordProtected: false, // Backend doesn't return this info
      maxViews: room.max_times_entered,
    },
    permissions: parsePermissions(room.permission),
    createdAt: room.created_at,
  };
}

/**
 * Convert backend RoomContent to frontend Message (for text content)
 */
export function backendContentToMessage(content: BackendRoomContent): Message {
  return {
    id: String(content.id),
    content: content.text || "",
    timestamp: content.created_at,
    fileName: content.file_name || undefined,
  };
}

/**
 * Convert backend RoomContent to frontend FileItem (for file content)
 */
export function backendContentToFileItem(
  content: BackendRoomContent,
): FileItem {
  const contentType = parseContentType(content.content_type);

  const typeMap: Record<number, FileItem["type"]> = {
    [ContentType.Image]: "image",
    [ContentType.File]: "document",
    [ContentType.Url]: "link",
  };

  return {
    id: String(content.id),
    name: content.file_name || "Unnamed",
    thumbnailUrl: null, // Backend doesn't provide thumbnails
    size: content.size || undefined,
    type: typeMap[contentType],
    url: content.url || undefined,
    mimeType: content.mime_type || undefined,
    createdAt: content.created_at,
  };
}
