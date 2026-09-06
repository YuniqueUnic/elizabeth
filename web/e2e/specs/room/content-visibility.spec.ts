import type { Page } from "@playwright/test";
import type { Actor } from "@serenity-js/core";

import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { CallElizabethApi } from "../../screenplay/abilities/CallElizabethApi.ability";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { primeRoomToken } from "../../screenplay/support/token-storage";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  OpenRoom,
  SendMessage,
  SaveMessages,
  UploadRoomFiles,
} from "../../screenplay/room/tasks/Room.tasks";
import { textFile } from "../../screenplay/support/test-data";

/**
 * 内容显隐（msg/file.visibility.manage）与权限边界。
 *
 * 覆盖：
 * - happy path：admin 隐藏/恢复消息与链接，reader 视图实时增减
 * - 服务端过滤：reader 的列表 API 不含隐藏内容
 * - 绕过尝试：reader 直链下载 → 404；reader 直接改可见性 → 403
 * - own 作用域：editor 只能隐藏/恢复自己创建的内容
 * - 会话吊销：admin 吊销后对方立即 401
 * - editor 席位上限（10 个）与 409
 */

const HIDDEN_MESSAGE_TEXT = "visibility-hidden-message";
const VISIBLE_MESSAGE_TEXT = "visibility-visible-message";

async function joinAsReader(
  createActor: (name: string) => Promise<{ actor: Actor; page: Page }>,
  api: CallElizabethApi,
  room: ProvisionedRoom,
  name: string,
): Promise<Page> {
  const handle = await createActor(name);
  const tokenInfo = await api.issueToken(room.name, { password: room.password });
  await primeRoomToken(handle.page, room.name, tokenInfo);
  await handle.actor.attemptsTo(OpenRoom(room.url));
  return handle.page;
}

test.describe("Content visibility and permission boundaries", () => {
  let room: ProvisionedRoom;
  let api: CallElizabethApi;
  let adminToken: string;

  test.beforeEach(async ({ actor, provisionRoom, request }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-visibility"),
    });
    api = CallElizabethApi.using(request);
    adminToken = room.tokenInfo!.token;
  });

  test("admin hides a message and the reader stops seeing it in UI and API", async ({
    actor,
    createActor,
  }) => {
    await actor.attemptsTo(OpenRoom(room.url));
    await actor.attemptsTo(SendMessage(HIDDEN_MESSAGE_TEXT));
    await actor.attemptsTo(SendMessage(VISIBLE_MESSAGE_TEXT));
    await actor.attemptsTo(SaveMessages());

    const adminList = await api.listMessages(room.name, adminToken);
    const hiddenTarget = adminList.items.find(
      (item) => item.text === HIDDEN_MESSAGE_TEXT,
    );
    expect(hiddenTarget, "target message should exist server-side").toBeTruthy();
    const contentId = Number(hiddenTarget!.id);

    const readerPage = await joinAsReader(createActor, api, room, "visibility-reader");
    await expect(readerPage.getByText(HIDDEN_MESSAGE_TEXT)).toBeVisible();

    // admin 隐藏 → 200
    expect(await api.setVisibility(room.name, contentId, true, adminToken)).toBe(200);

    // 服务端：reader 列表不含该消息
    const readerToken = (await api.issueToken(room.name)).token;
    const readerList = await api.listMessages(room.name, readerToken);
    expect(readerList.items.some((item: Record<string, unknown>) => item.id === contentId)).toBe(false);

    // UI：reader 实时消失（WS 事件驱动），其余消息不受影响
    await expect(readerPage.getByText(HIDDEN_MESSAGE_TEXT)).toBeHidden({ timeout: 15_000 });
    await expect(readerPage.getByText(VISIBLE_MESSAGE_TEXT)).toBeVisible();
  });

  test("reader cannot bypass visibility by direct download or direct toggle", async ({
    actor,
  }) => {
    await actor.attemptsTo(OpenRoom(room.url));
    // 走真实 UI 上传拿到可下载的文件内容
    await actor.attemptsTo(
      UploadRoomFiles(textFile("hidden-asset.txt", "hidden asset content")),
    );

    // 等文件出现在服务端列表并找到其 id
    let fileId: number | undefined;
    await expect
      .poll(async () => {
        const contents = await api.listContents(room.name, adminToken);
        const target = contents.items.find(
          (item: Record<string, unknown>) => item.file_name === "hidden-asset.txt",
        );
        fileId = target?.id as number | undefined;
        return Boolean(fileId);
      })
      .toBe(true);

    expect(await api.setVisibility(room.name, fileId!, true, adminToken)).toBe(200);

    const readerToken = (await api.issueToken(room.name)).token;

    // 列表过滤
    const contents = await api.listContents(room.name, readerToken);
    expect(contents.items.some((item: Record<string, unknown>) => item.id === fileId)).toBe(false);

    // 直链下载按「不存在」拒绝，堵住绕过
    expect(await api.downloadStatus(fileId!, readerToken)).toBe(404);

    // 直接改可见性 → 403
    expect(await api.setVisibility(room.name, fileId!, false, readerToken)).toBe(403);

    // admin 依旧可见可下载
    const adminContents = await api.listContents(room.name, adminToken);
    expect(adminContents.items.some((item: Record<string, unknown>) => item.id === fileId)).toBe(true);
    expect(await api.downloadStatus(fileId!, adminToken)).toBeLessThan(400);

    // 链接内容的隐藏同样生效（列表过滤即可区分）
    const link = await api.createUrlContent(
      room.name,
      { url: "https://example.com/hidden-link", name: "hidden-link" },
      adminToken,
    );
    expect(link.status).toBe(200);
    expect(await api.setVisibility(room.name, link.id!, true, adminToken)).toBe(200);
    const readerAfterLink = await api.listContents(room.name, readerToken);
    expect(
      readerAfterLink.items.some((item: Record<string, unknown>) => item.id === link.id),
    ).toBe(false);
  });

  test("editor with own scope manages own content but not others", async ({
    actor,
  }) => {
    await actor.attemptsTo(OpenRoom(room.url));

    const editor = await api.issueRoleToken(room.name, "editor", adminToken);
    expect(editor.roleKey).toBe("editor");

    // 自己的消息：隐藏 → 列表带 hidden 标记 → 恢复
    const own = await api.sendMessage(room.name, "editor-own-note", editor.token);
    expect(own.status).toBe(200);
    expect(await api.setVisibility(room.name, own.id!, true, editor.token)).toBe(200);

    const editorList = await api.listMessages(room.name, editor.token);
    const editorItems: Array<{ id?: number; hidden?: boolean }> = editorList.items;
    const ownItem = editorItems.find((item) => item.id === own.id);
    expect(ownItem?.hidden).toBe(true);

    // admin（any 作用域）依旧可见
    const adminList = await api.listMessages(room.name, adminToken);
    expect(adminList.items.some((item: Record<string, unknown>) => item.id === own.id)).toBe(true);

    expect(await api.setVisibility(room.name, own.id!, false, editor.token)).toBe(200);

    // 动别人的内容 → 403（消息域与文件域都要挡住）
    const adminMessage = await api.sendMessage(room.name, "admin-owned-note", adminToken);
    expect(await api.setVisibility(room.name, adminMessage.id!, true, editor.token)).toBe(403);
    const adminLink = await api.createUrlContent(
      room.name,
      { url: "https://example.com/admin-link", name: "admin-link" },
      adminToken,
    );
    expect(await api.setVisibility(room.name, adminLink.id!, true, editor.token)).toBe(403);
  });

  test("identity without visibility capability cannot hide anything", async ({
    actor,
  }) => {
    await actor.attemptsTo(OpenRoom(room.url));

    const reader = await api.issueToken(room.name);
    const message = await api.sendMessage(room.name, "reader-target", adminToken);
    expect(await api.setVisibility(room.name, message.id!, true, reader.token)).toBe(403);
  });

  test("revoked session loses access immediately", async ({ actor }) => {
    await actor.attemptsTo(OpenRoom(room.url));

    const editor = await api.issueRoleToken(room.name, "editor", adminToken);
    expect((await api.listMessages(room.name, editor.token)).status).toBe(200);

    const tokens = await api.listTokens(room.name, adminToken);
    const editorSession = tokens.find(
      (session: { jti: string; role_key?: string; revoked_at: string | null }) =>
        session.jti && !session.revoked_at && session.role_key === "editor",
    );
    expect(editorSession, "editor session should be listed").toBeTruthy();

    expect(await api.revokeToken(room.name, editorSession!.jti, adminToken)).toBe(200);

    // 立即失效
    expect((await api.listMessages(room.name, editor.token)).status).toBe(401);
  });

  test("editor seat limit is enforced at ten with conflict beyond", async ({
    actor,
  }) => {
    await actor.attemptsTo(OpenRoom(room.url));

    let issued = 0;
    let conflictSeen = false;
    for (let index = 0; index < 12; index += 1) {
      const status = await api.tryIssueRoleToken(room.name, "editor", adminToken);
      if (status === 200) {
        issued += 1;
      } else if (status === 409) {
        conflictSeen = true;
        break;
      } else {
        throw new Error(`unexpected issue status ${status}`);
      }
    }
    expect(issued).toBeLessThanOrEqual(10);
    expect(conflictSeen).toBe(true);
  });
});
