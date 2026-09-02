import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { FileCount, FileNames } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  DropFileIntoComposer,
  OpenRoom,
  PasteFileIntoComposer,
  PasteIntoComposer,
} from "../../screenplay/room/tasks/Room.tasks";
import {
  pngFile,
  textFile,
  uniqueRoomName,
} from "../../screenplay/support/test-data";

/**
 * 覆盖 issue #167：在消息编辑器中直接粘贴/拖放图片与文件，
 * 无需经由 File Manager 的独立上传入口，且上传结果存入 File Manager。
 */
test.describe("Message editor direct paste and drop into File Manager", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, page, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-editor-paste"),
    });
    await actor.attemptsTo(OpenRoom(room.url));

    await expect(RoomScreen.fileEmptyState(page)).toBeVisible();
  });

  test("stores a pasted image in the File Manager and inserts its markdown", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(PasteFileIntoComposer(pngFile("pasted-image.png")));

    await expect(RoomScreen.fileEmptyState(page)).toHaveCount(0);
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("pasted-image.png");
    await expect.poll(async () => actor.answer(FileCount())).toBe(1);

    // 编辑器收到图片 markdown 引用并渲染为图片节点
    await expect(RoomScreen.messageInput(page).locator("img")).toHaveCount(1);
  });

  test("stores a dropped file in the File Manager and inserts its link", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      DropFileIntoComposer(textFile("dropped-file.txt", "dropped body")),
    );

    await expect(RoomScreen.fileEmptyState(page)).toHaveCount(0);
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("dropped-file.txt");
    await expect.poll(async () => actor.answer(FileCount())).toBe(1);

    // 编辑器收到文件链接 markdown
    await expect(RoomScreen.messageInput(page)).toContainText("dropped-file.txt");
  });

  test("keeps plain-text paste out of the File Manager", async ({ actor, page }) => {
    await actor.attemptsTo(PasteIntoComposer("plain text paste only"));

    await expect(RoomScreen.messageInput(page)).toContainText("plain text paste only");
    await expect(RoomScreen.fileEmptyState(page)).toBeVisible();
    await expect.poll(async () => actor.answer(FileCount())).toBe(0);
  });
});
