import { api } from "../lib/utils/api";

export type PolicyMode = "off" | "reusable" | "one_time";

export interface PolicyResponse {
  id: number;
  content_id: number;
  mode: PolicyMode;
  max_downloads?: number | null;
  download_count: number;
  total_codes: number;
  remaining_codes: number;
  used_codes: number;
}

export interface SetPolicyRequest {
  mode: PolicyMode;
  max_downloads?: number | null;
  codes?: string[];
}

export interface GenerateCodesRequest {
  count: number;
  is_reusable: boolean;
}

export interface GenerateCodesResponse {
  codes: string[];
}

export interface RedeemRequest {
  code: string;
}

export interface RedeemResponse {
  ticket: string;
}

export async function getPolicy(roomName: string, contentId: string): Promise<PolicyResponse | null> {
  const result = await api.get<PolicyResponse | null>(`/api/v1/rooms/${roomName}/contents/${contentId}/policy`);
  // Backend returns null when no policy is configured (defaults to "off")
  return result ?? null;
}

export async function setPolicy(roomName: string, contentId: string, data: SetPolicyRequest): Promise<PolicyResponse> {
  return await api.put(`/api/v1/rooms/${roomName}/contents/${contentId}/policy`, data);
}

export async function generateCodes(roomName: string, contentId: string, data: GenerateCodesRequest): Promise<GenerateCodesResponse> {
  return await api.post(`/api/v1/rooms/${roomName}/contents/${contentId}/policy/generate-codes`, data);
}

export async function redeemCode(roomName: string, contentId: string, data: RedeemRequest): Promise<RedeemResponse> {
  return await api.post(`/api/v1/rooms/${roomName}/contents/${contentId}/redeem`, data);
}
