import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";

export interface PublicRoomExpiryConfig {
  allowed_ages_seconds: number[];
  default_age_seconds: number;
}

export interface PublicConfigResponse {
  room: {
    expiry: PublicRoomExpiryConfig;
  };
}

export function getPublicConfig(): Promise<PublicConfigResponse> {
  return api.get<PublicConfigResponse>(API_ENDPOINTS.publicConfig, undefined, {
    skipTokenInjection: true,
  });
}
