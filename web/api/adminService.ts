import { ADMIN_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import type {
  AdminConfigResponse,
  AdminCredentialView,
  AdminMintIdentityCodeRequest,
  AdminRoomDetailResponse,
  AdminRoomListResponse,
  AdminStatsResponse,
  AdminStorageResponse,
  CreateRoomIdentityCodeResponse,
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

export function updateRuntimeConfig(
  adminToken: string,
  update: {
    disallow_search_indexing?: boolean;
    room_default_max_size?: number;
    room_default_max_times_entered?: number;
    session_ttl_seconds?: number;
    upload_reservation_ttl_seconds?: number;
    room_default_role_key?: string;
  },
): Promise<AdminConfigResponse> {
  return api.put<AdminConfigResponse>(
    ADMIN_ENDPOINTS.runtimeConfig,
    update,
    adminOptions(adminToken),
  );
}

/** 平台运维语义的房间设置更新（复用房间设置校验，服务端强制）。 */
export function updateAdminRoom(
  adminToken: string,
  name: string,
  update: {
    max_size?: number;
    max_times_entered?: number;
    default_role_key?: string;
    password?: string;
    remove_password?: boolean;
  },
): Promise<AdminRoomDetailResponse> {
  return api.put<AdminRoomDetailResponse>(
    ADMIN_ENDPOINTS.roomUpdate(name),
    update,
    adminOptions(adminToken),
  );
}

/** 铸造房间身份码；code 留空由服务端生成，明文仅此一次返回。 */
export function mintAdminIdentityCode(
  adminToken: string,
  name: string,
  mint: AdminMintIdentityCodeRequest,
): Promise<CreateRoomIdentityCodeResponse> {
  return api.post<CreateRoomIdentityCodeResponse>(
    ADMIN_ENDPOINTS.roomIdentityCodes(name),
    mint,
    adminOptions(adminToken),
  );
}

/** 轮换平台管理凭证（进程内覆盖，重启回退环境引导值）。 */
export function updateAdminCredential(
  adminToken: string,
  token: string,
): Promise<AdminCredentialView> {
  return api.put<AdminCredentialView>(
    ADMIN_ENDPOINTS.credential,
    { token },
    adminOptions(adminToken),
  );
}
