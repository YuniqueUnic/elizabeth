import type { APIRequestContext } from "@playwright/test";

import { API_BASE_URL } from "./constants";

/** 后端 naive datetime 为 UTC；纳秒精度需截断到毫秒才能被 Date 解析。 */
export function parseUtcMillis(value: string): number {
  const normalized = value.replace(/(\.\d{3})\d+$/, "$1");
  return Date.parse(value.includes("T") ? `${normalized}Z` : normalized);
}

/** 房间的服务端视图；房间不可读时直接失败。 */
export async function roomView(
  request: APIRequestContext,
  roomName: string,
): Promise<{
  max_size: number;
  default_role_key: string;
  upload_file_type: { mode: string; extensions: string[] };
  expire_at: string | null;
  created_at: string;
}> {
  const response = await request.get(
    `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}`,
  );
  if (!response.ok()) {
    throw new Error(`Room ${roomName} is not readable: ${response.status()}`);
  }
  return response.json();
}

/** 房间距服务端截止时刻的剩余秒数；房间无截止时刻时直接失败。 */
export async function remainingSeconds(
  request: APIRequestContext,
  roomName: string,
): Promise<number> {
  const { expire_at: expireAt } = await roomView(request, roomName);
  if (!expireAt) {
    throw new Error(`Room ${roomName} has no server-side deadline`);
  }
  return Math.round((parseUtcMillis(expireAt) - Date.now()) / 1000);
}

/** 部署配置允许的房间时长与新建房间的默认时长。 */
export async function roomExpiryPolicy(
  request: APIRequestContext,
): Promise<{ allowedAges: number[]; defaultAge: number }> {
  const response = await request.get(`${API_BASE_URL}/config`);
  if (!response.ok()) {
    throw new Error(`Public config unavailable: ${response.status()}`);
  }
  const {
    allowed_ages_seconds: allowedAges,
    default_age_seconds: defaultAge,
  } = (await response.json()).room.expiry;
  return { allowedAges, defaultAge };
}
