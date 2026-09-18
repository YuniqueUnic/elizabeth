import { ADMIN_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import type {
  AdminApiKeyCreateRequest,
  AdminApiKeyCreateResponse,
  AdminApiKeyView,
  AdminConfigResponse,
  AdminLoginResponse,
  AdminMintIdentityCodeRequest,
  AdminRoomDetailResponse,
  AdminRoomListResponse,
  AdminSessionView,
  AdminStatsResponse,
  AdminStorageResponse,
  CreateRoomIdentityCodeResponse,
  RoomExpiryOverride,
  UploadFileTypePolicy,
} from "../types/generated/api.types";

/**
 * 平台管理后台服务。
 * 会话令牌（登录签发的 JWT）或 API key 只作为请求头由服务端校验，
 * 前端不做任何授权判定。
 */
function adminOptions(sessionToken: string) {
  return {
    skipTokenInjection: true,
    headers: { Authorization: `Bearer ${sessionToken}` },
  } as const;
}

/** 管理员登录：换取会话令牌（JWT）。 */
export function adminLogin(
  username: string,
  password: string,
): Promise<AdminLoginResponse> {
  return api.post<AdminLoginResponse>(ADMIN_ENDPOINTS.login, {
    username,
    password,
  });
}

/** 校验会话是否仍然有效。 */
export function adminMe(sessionToken: string): Promise<AdminSessionView> {
  return api.get<AdminSessionView>(
    ADMIN_ENDPOINTS.me,
    undefined,
    adminOptions(sessionToken),
  );
}

/** 修改当前管理员账号密码；服务端将使该账号所有既有会话失效。 */
export function adminChangePassword(
  sessionToken: string,
  update: { current_password: string; new_password: string },
): Promise<AdminSessionView> {
  return api.put<AdminSessionView>(
    ADMIN_ENDPOINTS.password,
    update,
    adminOptions(sessionToken),
  );
}

export function getAdminStats(sessionToken: string): Promise<AdminStatsResponse> {
  return api.get<AdminStatsResponse>(
    ADMIN_ENDPOINTS.stats,
    undefined,
    adminOptions(sessionToken),
  );
}

export function listAdminRooms(
  sessionToken: string,
  params: { q?: string; limit?: number; offset?: number } = {},
): Promise<AdminRoomListResponse> {
  const query: Record<string, string | number | boolean> = {};
  if (params.q) query.q = params.q;
  if (params.limit !== undefined) query.limit = params.limit;
  if (params.offset !== undefined) query.offset = params.offset;
  return api.get<AdminRoomListResponse>(
    ADMIN_ENDPOINTS.rooms,
    query,
    adminOptions(sessionToken),
  );
}

export function getAdminRoomDetail(
  sessionToken: string,
  name: string,
): Promise<AdminRoomDetailResponse> {
  return api.get<AdminRoomDetailResponse>(
    ADMIN_ENDPOINTS.roomDetail(name),
    undefined,
    adminOptions(sessionToken),
  );
}

export function deleteAdminRoom(sessionToken: string, name: string): Promise<void> {
  return api.delete<void>(
    ADMIN_ENDPOINTS.roomDelete(name),
    undefined,
    adminOptions(sessionToken),
  );
}

export function getAdminStorage(sessionToken: string): Promise<AdminStorageResponse> {
  return api.get<AdminStorageResponse>(
    ADMIN_ENDPOINTS.storage,
    undefined,
    adminOptions(sessionToken),
  );
}

export function getAdminConfig(sessionToken: string): Promise<AdminConfigResponse> {
  return api.get<AdminConfigResponse>(
    ADMIN_ENDPOINTS.config,
    undefined,
    adminOptions(sessionToken),
  );
}

export function updateRuntimeConfig(
  sessionToken: string,
  update: {
    disallow_search_indexing?: boolean;
    room_default_max_size?: number;
    room_default_max_times_entered?: number;
    session_ttl_seconds?: number;
    upload_reservation_ttl_seconds?: number;
    room_default_role_key?: string;
    /** 房间有效期策略覆盖；空允许列表 = 清除覆盖 */
    room_expiry?: RoomExpiryOverride;
  },
): Promise<AdminConfigResponse> {
  return api.put<AdminConfigResponse>(
    ADMIN_ENDPOINTS.runtimeConfig,
    update,
    adminOptions(sessionToken),
  );
}

/** 平台运维语义的房间设置更新（复用房间设置校验，服务端强制）。 */
export function updateAdminRoom(
  sessionToken: string,
  name: string,
  update: {
    max_size?: number;
    max_times_entered?: number;
    default_role_key?: string;
    /** 房间有效期（秒）；必须属于部署配置允许的期限 */
    age_seconds?: number;
    /** 上传文件类型策略（any/allow/deny + 扩展名列表） */
    upload_file_type?: UploadFileTypePolicy;
    password?: string;
    remove_password?: boolean;
  },
): Promise<AdminRoomDetailResponse> {
  return api.put<AdminRoomDetailResponse>(
    ADMIN_ENDPOINTS.roomUpdate(name),
    update,
    adminOptions(sessionToken),
  );
}

/** 铸造房间身份码；code 留空由服务端生成，明文仅此一次返回。 */
export function mintAdminIdentityCode(
  sessionToken: string,
  name: string,
  mint: AdminMintIdentityCodeRequest,
): Promise<CreateRoomIdentityCodeResponse> {
  return api.post<CreateRoomIdentityCodeResponse>(
    ADMIN_ENDPOINTS.roomIdentityCodes(name),
    mint,
    adminOptions(sessionToken),
  );
}

/** 列出未吊销的管理 API key（不回显明文）。 */
export function listAdminApiKeys(
  sessionToken: string,
): Promise<AdminApiKeyView[]> {
  return api.get<AdminApiKeyView[]>(
    ADMIN_ENDPOINTS.apiKeys,
    undefined,
    adminOptions(sessionToken),
  );
}

/** 创建管理 API key；明文 secret 仅此一次返回。 */
export function createAdminApiKey(
  sessionToken: string,
  request: AdminApiKeyCreateRequest,
): Promise<AdminApiKeyCreateResponse> {
  return api.post<AdminApiKeyCreateResponse>(
    ADMIN_ENDPOINTS.apiKeys,
    request,
    adminOptions(sessionToken),
  );
}

/** 吊销管理 API key。 */
export function revokeAdminApiKey(
  sessionToken: string,
  id: number,
): Promise<void> {
  return api.delete<void>(
    ADMIN_ENDPOINTS.apiKeyDelete(id),
    undefined,
    adminOptions(sessionToken),
  );
}
