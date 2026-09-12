import { ADMIN_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import type {
  AdminConfigResponse,
  AdminRoomDetailResponse,
  AdminRoomListResponse,
  AdminStatsResponse,
  AdminStorageResponse,
} from "../types/generated/api.types";

/**
 * 平台管理后台服务（issue #196）。
 * admin token 仅作为请求头由服务端校验，不在前端做任何授权判定。
 */
function adminOptions(adminToken: string) {
  return {
    skipTokenInjection: true,
    headers: { "X-Elizabeth-Admin-Token": adminToken },
  } as const;
}

export function getAdminStats(adminToken: string): Promise<AdminStatsResponse> {
  return api.get<AdminStatsResponse>(
    ADMIN_ENDPOINTS.stats,
    undefined,
    adminOptions(adminToken),
  );
}

export function listAdminRooms(
  adminToken: string,
  params: { q?: string; limit?: number; offset?: number } = {},
): Promise<AdminRoomListResponse> {
  const query: Record<string, string | number | boolean> = {};
  if (params.q) query.q = params.q;
  if (params.limit !== undefined) query.limit = params.limit;
  if (params.offset !== undefined) query.offset = params.offset;
  return api.get<AdminRoomListResponse>(
    ADMIN_ENDPOINTS.rooms,
    query,
    adminOptions(adminToken),
  );
}

export function getAdminRoomDetail(
  adminToken: string,
  name: string,
): Promise<AdminRoomDetailResponse> {
  return api.get<AdminRoomDetailResponse>(
    ADMIN_ENDPOINTS.roomDetail(name),
    undefined,
    adminOptions(adminToken),
  );
}

export function deleteAdminRoom(adminToken: string, name: string): Promise<void> {
  return api.delete<void>(
    ADMIN_ENDPOINTS.roomDelete(name),
    undefined,
    adminOptions(adminToken),
  );
}

export function getAdminStorage(adminToken: string): Promise<AdminStorageResponse> {
  return api.get<AdminStorageResponse>(
    ADMIN_ENDPOINTS.storage,
    undefined,
    adminOptions(adminToken),
  );
}

export function getAdminConfig(adminToken: string): Promise<AdminConfigResponse> {
  return api.get<AdminConfigResponse>(
    ADMIN_ENDPOINTS.config,
    undefined,
    adminOptions(adminToken),
  );
}
