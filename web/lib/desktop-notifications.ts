"use client";

import type { ContentEventPayload } from "@/lib/hooks/use-room-events";
import { ContentType, parseContentType } from "@/lib/types";

export type DesktopNotificationKind = "message" | "file" | "link";
export type DesktopNotificationAction = "created" | "updated" | "deleted";
export type DesktopNotificationPermission = NotificationPermission | "unsupported";

export type DesktopNotificationTypes = Record<
  DesktopNotificationKind,
  Record<DesktopNotificationAction, boolean>
>;

export const desktopNotificationKinds: DesktopNotificationKind[] = [
  "message",
  "file",
  "link",
];

export const desktopNotificationActions: DesktopNotificationAction[] = [
  "created",
  "updated",
  "deleted",
];

export function createDefaultDesktopNotificationTypes(): DesktopNotificationTypes {
  return {
    message: { created: true, updated: true, deleted: true },
    file: { created: true, updated: true, deleted: true },
    link: { created: true, updated: true, deleted: true },
  };
}

export function normalizeDesktopNotificationTypes(
  value: Partial<DesktopNotificationTypes> | undefined,
): DesktopNotificationTypes {
  const defaults = createDefaultDesktopNotificationTypes();

  return {
    message: { ...defaults.message, ...value?.message },
    file: { ...defaults.file, ...value?.file },
    link: { ...defaults.link, ...value?.link },
  };
}

export function isBrowserNotificationSupported(): boolean {
  return typeof window !== "undefined" &&
    "Notification" in window &&
    window.isSecureContext;
}

export function getBrowserNotificationPermission(): DesktopNotificationPermission {
  if (!isBrowserNotificationSupported()) return "unsupported";
  return window.Notification.permission;
}

export async function requestBrowserNotificationPermission(): Promise<DesktopNotificationPermission> {
  if (!isBrowserNotificationSupported()) return "unsupported";

  try {
    return await window.Notification.requestPermission();
  } catch {
    return "denied";
  }
}

export function getContentNotificationKind(
  payload: ContentEventPayload,
): DesktopNotificationKind | null {
  const contentType = parseContentType(payload.content_type);

  switch (contentType) {
    case ContentType.Text:
      return "message";
    case ContentType.Url:
      return "link";
    case ContentType.Image:
    case ContentType.File:
      return "file";
    default:
      return null;
  }
}

export function getContentNotificationSubject(
  payload: ContentEventPayload,
  kind: DesktopNotificationKind,
): string {
  const raw = kind === "message"
    ? payload.text
    : payload.file_name ?? payload.text;

  const normalized = (raw ?? "").replace(/\s+/g, " ").trim();
  if (normalized.length <= 80) return normalized;
  return `${normalized.slice(0, 77)}...`;
}

export function showContentDesktopNotification({
  enabled,
  types,
  payload,
  action,
  title,
  body,
  roomName,
}: {
  enabled: boolean;
  types: DesktopNotificationTypes;
  payload: ContentEventPayload;
  action: DesktopNotificationAction;
  title: string;
  body: string;
  roomName: string;
}): boolean {
  if (!enabled || getBrowserNotificationPermission() !== "granted") {
    return false;
  }

  const kind = getContentNotificationKind(payload);
  if (!kind || !types[kind]?.[action]) {
    return false;
  }

  try {
    const contentId = payload.content_id ?? "unknown";
    new window.Notification(title, {
      body,
      tag: `elizabeth:${roomName}:${kind}:${action}:${contentId}`,
    });
    return true;
  } catch {
    return false;
  }
}
