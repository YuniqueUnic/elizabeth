"use client";

import type { ContentEventPayload } from "@/lib/hooks/use-room-events";
import { ContentType, parseContentType } from "@/lib/types";

export type ContentDesktopNotificationKind = "message" | "file" | "link";
export type RoomDesktopNotificationAction =
  | "address_changed"
  | "permissions_changed"
  | "settings_changed";
export type DesktopNotificationKind = ContentDesktopNotificationKind | "room";
export type DesktopNotificationAction =
  | "created"
  | "updated"
  | "deleted"
  | RoomDesktopNotificationAction;
export type DesktopNotificationPermission = NotificationPermission | "unsupported";

export type DesktopNotificationTypes = Record<
  DesktopNotificationKind,
  Record<DesktopNotificationAction, boolean>
>;

type PartialDesktopNotificationTypes = Partial<
  Record<
    DesktopNotificationKind,
    Partial<Record<DesktopNotificationAction, boolean>>
  >
>;

export const desktopNotificationKinds: DesktopNotificationKind[] = [
  "message",
  "file",
  "link",
  "room",
];

export const desktopNotificationActions: DesktopNotificationAction[] = [
  "created",
  "updated",
  "deleted",
  "address_changed",
  "permissions_changed",
  "settings_changed",
];

export const desktopNotificationActionsByKind: Record<
  DesktopNotificationKind,
  DesktopNotificationAction[]
> = {
  message: ["created", "updated", "deleted"],
  file: ["created", "deleted"],
  link: ["created", "deleted"],
  room: ["address_changed", "permissions_changed", "settings_changed"],
};

function createDisabledDesktopNotificationActions(): Record<
  DesktopNotificationAction,
  boolean
> {
  return {
    created: false,
    updated: false,
    deleted: false,
    address_changed: false,
    permissions_changed: false,
    settings_changed: false,
  };
}

export function createDefaultDesktopNotificationTypes(): DesktopNotificationTypes {
  return {
    message: {
      ...createDisabledDesktopNotificationActions(),
      created: true,
      updated: true,
      deleted: true,
    },
    file: {
      ...createDisabledDesktopNotificationActions(),
      created: true,
      deleted: true,
    },
    link: {
      ...createDisabledDesktopNotificationActions(),
      created: true,
      deleted: true,
    },
    room: {
      ...createDisabledDesktopNotificationActions(),
      address_changed: true,
      permissions_changed: true,
      settings_changed: true,
    },
  };
}

export function normalizeDesktopNotificationTypes(
  value: PartialDesktopNotificationTypes | undefined,
): DesktopNotificationTypes {
  const defaults = createDefaultDesktopNotificationTypes();
  const normalized = {
    message: { ...defaults.message, ...value?.message },
    file: { ...defaults.file, ...value?.file },
    link: { ...defaults.link, ...value?.link },
    room: { ...defaults.room, ...value?.room },
  };

  for (const kind of desktopNotificationKinds) {
    const supported = new Set(desktopNotificationActionsByKind[kind]);
    for (const action of desktopNotificationActions) {
      if (!supported.has(action)) normalized[kind][action] = false;
    }
  }

  return normalized;
}

export function isDesktopNotificationActionSupported(
  kind: DesktopNotificationKind,
  action: DesktopNotificationAction,
): boolean {
  return desktopNotificationActionsByKind[kind].includes(action);
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
): ContentDesktopNotificationKind | null {
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
  kind: ContentDesktopNotificationKind,
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
  if (
    !kind ||
    !isDesktopNotificationActionSupported(kind, action) ||
    !types[kind]?.[action]
  ) {
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

export function showRoomDesktopNotification({
  enabled,
  types,
  action,
  title,
  body,
  roomName,
  tagSubject,
}: {
  enabled: boolean;
  types: DesktopNotificationTypes;
  action: RoomDesktopNotificationAction;
  title: string;
  body: string;
  roomName: string;
  tagSubject: string;
}): boolean {
  if (!enabled || getBrowserNotificationPermission() !== "granted") {
    return false;
  }

  if (
    !isDesktopNotificationActionSupported("room", action) ||
    !types.room?.[action]
  ) {
    return false;
  }

  try {
    new window.Notification(title, {
      body,
      tag: `elizabeth:${roomName}:room:${action}:${tagSubject}`,
    });
    return true;
  } catch {
    return false;
  }
}
