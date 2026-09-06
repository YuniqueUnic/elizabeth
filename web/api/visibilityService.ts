/**
 * Content Visibility Service
 *
 * 消息 / 文件 / 链接共用的「显示 / 隐藏」开关。
 * 服务端按内容类型校验 msg.visibility.manage 或 file.visibility.manage 能力。
 */

import { API_ENDPOINTS } from "../lib/config";
import { api } from "../lib/utils/api";
import { getValidToken } from "./authService";
import type { BackendRoomContent } from "../lib/types";
import type { UpdateContentResponse } from "../types/generated/api.types";

export async function setContentVisibility(
  roomName: string,
  contentId: string,
  hidden: boolean,
  token?: string,
): Promise<BackendRoomContent> {
  const authToken = token || (await getValidToken(roomName));
  if (!authToken) {
    throw new Error("Authentication required to change content visibility");
  }

  const response = await api.put<UpdateContentResponse>(
    API_ENDPOINTS.content.visibility(roomName, contentId),
    { hidden },
    { token: authToken },
  );

  return response.updated;
}
