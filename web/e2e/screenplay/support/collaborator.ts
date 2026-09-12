import type { Page } from "@playwright/test";
import type { Actor } from "@serenity-js/core";

import { CallElizabethApi } from "../abilities/CallElizabethApi.ability";
import type { ProvisionedRoom } from "./constants";
import { primeRoomToken } from "./token-storage";
import { OpenRoom } from "../room/tasks/Room.tasks";

export interface ActorHandle {
  actor: Actor;
  page: Page;
}

/**
 * 让协作者以指定角色的身份码加入房间（产品流程：admin 分发身份码给协作成员）。
 * 第二 actor 的浏览器上下文没有预置 token，直接 OpenRoom 只会拿到 reader，
 * 无保存/编辑能力，会让依赖发消息的用例全部卡死。
 */
export async function joinAsRole(
  createActor: (name: string) => Promise<ActorHandle>,
  api: CallElizabethApi,
  room: ProvisionedRoom,
  name: string,
  adminToken: string,
  role: "editor" | "admin" = "editor",
): Promise<ActorHandle> {
  const handle = await createActor(name);
  const roleToken = await api.issueRoleToken(room.name, role, adminToken);
  await primeRoomToken(handle.page, room.name, roleToken);
  await handle.actor.attemptsTo(OpenRoom(room.url));
  return handle;
}

/** editor 协作者（最常用）。 */
export async function joinAsEditor(
  createActor: (name: string) => Promise<ActorHandle>,
  api: CallElizabethApi,
  room: ProvisionedRoom,
  name: string,
  adminToken: string,
): Promise<ActorHandle> {
  return joinAsRole(createActor, api, room, name, adminToken, "editor");
}
