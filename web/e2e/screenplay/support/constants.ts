export const APP_BASE_URL = process.env.PLAYWRIGHT_BASE_URL ??
  "http://127.0.0.1:4093";

export const API_BASE_URL = process.env.API_BASE_URL ??
  `${APP_BASE_URL}/api/v1`;

export const TOKEN_STORAGE_KEY = "elizabeth_tokens";

export interface RoomTokenInfo {
  token: string;
  expiresAt: string;
  refreshToken?: string;
}

export interface ProvisionedRoom {
  name: string;
  url: string;
  password?: string;
  tokenInfo?: RoomTokenInfo;
}
