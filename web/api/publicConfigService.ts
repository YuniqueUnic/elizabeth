import { API_ENDPOINTS } from "../lib/config";
import type { PublicConfigResponse } from "../types/generated/PublicConfigResponse";
import { api } from "../lib/utils/api";

export function getPublicConfig(): Promise<PublicConfigResponse> {
  return api.get<PublicConfigResponse>(API_ENDPOINTS.publicConfig, undefined, {
    skipTokenInjection: true,
  });
}
