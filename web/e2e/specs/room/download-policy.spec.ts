import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { FileNames } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { tRoom } from "../../screenplay/support/i18n";
import {
  CloseFilePreview,
  ConfigureDownloadPolicy,
  DownloadPreviewedRoomFile,
  OpenRoom,
  PreviewRoomFile,
  RedeemFileAccessCode,
  UploadRoomFiles,
} from "../../screenplay/room/tasks/Room.tasks";
import { textFile, uniqueRoomName } from "../../screenplay/support/test-data";

/**
 * 覆盖 issue #166 / PR #169 的用户侧下载策略与访问码保护流程：
 *   点击文件卡片
 *     ├── 未受保护 (mode: off)          → 直接打开「详情 / 预览」弹窗
 *     └── 受保护 (reusable / one_time)  → 拦截点击，弹出访问码校验弹窗 (RedeemDialog)
 *         → POST /redeem 校验通过，签发一次性 short-lived ticket
 *         → 携带 ticket 解锁并打开「详情 / 预览」弹窗
 *         → 点击下载，携带 ticket 成功下载受保护文件流
 */
test.describe("Room file download policy and access codes", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-download-policy"),
    });
    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("opens the preview dialog directly for unprotected files (mode: off)", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("open-file.txt", "no protection here")),
    );
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("open-file.txt");

    await actor.attemptsTo(PreviewRoomFile("open-file.txt"));

    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewTitle(page)).toHaveText("open-file.txt");
    await expect(RoomScreen.redeemDialog(page)).toHaveCount(0);
  });

  test("configures a reusable access code and marks the file card as protected", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("guarded.txt", "guarded contents")),
    );
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("guarded.txt");

    await actor.attemptsTo(
      ConfigureDownloadPolicy("guarded.txt", { mode: "reusable", reusableCode: "VIP666" }),
    );

    await expect(RoomScreen.fileProtectedBadge(page, "guarded.txt")).toBeVisible();
  });

  test("intercepts protected file clicks with the redeem dialog and rejects wrong codes", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("intercepted.txt", "intercepted contents")),
      ConfigureDownloadPolicy("intercepted.txt", { mode: "reusable", reusableCode: "VIP666" }),
    );
    await expect(RoomScreen.fileProtectedBadge(page, "intercepted.txt")).toBeVisible();

    // 受保护卡片点击被拦截：弹出访问码校验弹窗，而不是直接打开预览
    await actor.attemptsTo(PreviewRoomFile("intercepted.txt"));

    await expect(RoomScreen.redeemDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewDownloadButton(page)).toHaveCount(0);

    await actor.attemptsTo(RedeemFileAccessCode("WRONG-CODE"));

    // 后端统一拒绝消息会原样展示在弹窗内（本地化 fallback 为 downloadPolicy.redeemFailed）
    await expect(RoomScreen.redeemError(page)).toContainText(
      /Invalid, expired, or depleted code|验证失败，访问码无效或已用尽/,
    );
    await expect(RoomScreen.redeemDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewDownloadButton(page)).toHaveCount(0);
  });

  test("unlocks the preview with a valid code and downloads through the issued ticket", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("ticketed.txt", "ticketed body text")),
      ConfigureDownloadPolicy("ticketed.txt", { mode: "reusable", reusableCode: "VIP666" }),
    );
    await expect(RoomScreen.fileProtectedBadge(page, "ticketed.txt")).toBeVisible();

    await actor.attemptsTo(PreviewRoomFile("ticketed.txt"));
    await expect(RoomScreen.redeemDialog(page)).toBeVisible();

    await actor.attemptsTo(RedeemFileAccessCode("VIP666"));

    // ticket 签发后解锁并打开「详情 / 预览」弹窗
    await expect(RoomScreen.redeemDialog(page)).toHaveCount(0);
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewTitle(page)).toHaveText("ticketed.txt");
    await expect(RoomScreen.filePreviewDialog(page)).not.toContainText(
      tRoom("downloadPolicy.previewProtectedMessage"),
    );
    await expect(RoomScreen.filePreviewDialog(page)).toContainText("ticketed body text");

    // 点击下载：携带 ticket 成功下载受保护文件流
    const downloadPromise = page.waitForEvent("download");
    const downloadRequest = page.waitForResponse((response) =>
      response.url().includes("/api/v1/contents/") &&
      response.url().includes("ticket="),
    );
    await actor.attemptsTo(DownloadPreviewedRoomFile());

    const download = await downloadPromise;
    const response = await downloadRequest;
    expect(response.status()).toBe(200);
    expect(download.suggestedFilename()).toBe("ticketed.txt");
  });

  test("burns one-time codes after a single successful redeem", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("one-shot.txt", "one shot body")),
      ConfigureDownloadPolicy("one-shot.txt", {
        mode: "one_time",
        oneTimeCodes: ["ONCE-AAA", "ONCE-BBB"],
      }),
    );
    await expect(RoomScreen.fileProtectedBadge(page, "one-shot.txt")).toBeVisible();

    // 第一个一次性码兑换成功并解锁预览
    await actor.attemptsTo(PreviewRoomFile("one-shot.txt"), RedeemFileAccessCode("ONCE-AAA"));
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await actor.attemptsTo(CloseFilePreview());

    // 同一个码已被核销，再次兑换被拒绝
    await actor.attemptsTo(PreviewRoomFile("one-shot.txt"), RedeemFileAccessCode("ONCE-AAA"));
    await expect(RoomScreen.redeemError(page)).toBeVisible();
    await expect(RoomScreen.filePreviewDownloadButton(page)).toHaveCount(0);

    // 码池中的下一个码仍然可用
    await actor.attemptsTo(RedeemFileAccessCode("ONCE-BBB"));
    await expect(RoomScreen.redeemDialog(page)).toHaveCount(0);
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewTitle(page)).toHaveText("one-shot.txt");
  });

  test("removes protection after switching the policy back to off", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("reopened.txt", "reopen me freely")),
      ConfigureDownloadPolicy("reopened.txt", { mode: "reusable", reusableCode: "VIP666" }),
    );
    await expect(RoomScreen.fileProtectedBadge(page, "reopened.txt")).toBeVisible();

    await actor.attemptsTo(ConfigureDownloadPolicy("reopened.txt", { mode: "off" }));

    await expect(RoomScreen.fileProtectedBadge(page, "reopened.txt")).toHaveCount(0);

    await actor.attemptsTo(PreviewRoomFile("reopened.txt"));

    await expect(RoomScreen.redeemDialog(page)).toHaveCount(0);
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewTitle(page)).toHaveText("reopened.txt");
  });
});
