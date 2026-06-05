import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { test, expect } from "../../screenplay/fixtures/screenplay.fixture";
import {
  ClipboardContents,
  FileCount,
  FileNames,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { tRoom } from "../../screenplay/support/i18n";
import {
  CopyPreviewRoomFileLink,
  CopyPreviewRoomFileMarkdown,
  DeleteRoomFile,
  DownloadPreviewedRoomFile,
  InsertPreviewRoomFileMarkdown,
  OpenRoom,
  PreviewRoomFile,
  UploadRoomFiles,
} from "../../screenplay/room/tasks/Room.tasks";
import {
  markdownFile,
  pdfFile,
  pngFile,
  textFile,
} from "../../screenplay/support/test-data";

function escapeForRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

test.describe("Room files and preview modal", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, page, provisionRoom }) => {
    room = await provisionRoom({ actor });
    await actor.attemptsTo(OpenRoom(room.url));

    await expect(RoomScreen.fileEmptyState(page)).toBeVisible();
  });

  test("uploads files, hides the empty state, and supports selecting all files", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(
        textFile("alpha.txt", "alpha"),
        markdownFile("notes.md", "# Notes\n\nHello"),
      ),
    );

    await expect(RoomScreen.fileEmptyState(page)).toHaveCount(0);
    await expect.poll(async () => actor.answer(FileCount())).toBe(2);
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("alpha.txt");
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("notes.md");

    await RoomScreen.fileSelectAllButton(page).click();
    await expect(page.getByText(tRoom("fileManager.selectedCount", { count: 2 }))).toBeVisible();
  });

  test("copies a public absolute download URL from the preview modal", async ({
    actor,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("public-link.txt", "download me")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("public-link.txt");

    await actor.attemptsTo(
      PreviewRoomFile("public-link.txt"),
      CopyPreviewRoomFileLink(),
    );

    const clipboard = await actor.answer(ClipboardContents());
    const base = escapeForRegex(room.url.replace(/\/[^/]+$/, ""));

    expect(clipboard).toMatch(
      new RegExp(`^${base}/api/v1/contents/\\d+\\?token=`),
    );
    expect(clipboard).toContain(`token=${room.tokenInfo?.token}`);
  });

  test("copies markdown using the same public absolute download URL", async ({
    actor,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(pngFile("pixel.png")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("pixel.png");

    await actor.attemptsTo(
      PreviewRoomFile("pixel.png"),
      CopyPreviewRoomFileMarkdown(),
    );

    const clipboard = await actor.answer(ClipboardContents());
    const base = escapeForRegex(room.url.replace(/\/[^/]+$/, ""));

    expect(clipboard).toMatch(
      new RegExp(`^!\\[\\]\\(${base}/api/v1/contents/\\d+\\?token=`),
    );
    expect(clipboard).toContain(`token=${room.tokenInfo?.token}`);
  });

  test("downloads the previewed file instead of silently failing", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(pngFile("downloadable.png")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("downloadable.png");

    await actor.attemptsTo(
      PreviewRoomFile("downloadable.png"),
    );

    const downloadPromise = page.waitForEvent("download");
    await actor.attemptsTo(
      DownloadPreviewedRoomFile(),
    );
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toBe("downloadable.png");

    // Verify the downloaded file has actual content (not an empty blob)
    const filePath = await download.path();
    expect(filePath).toBeTruthy();
  });

  test("uploads and previews a PDF file without errors", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(pdfFile("test-document")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("test-document.pdf");

    await actor.attemptsTo(
      PreviewRoomFile("test-document.pdf"),
    );

    // Verify the preview dialog opened
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();

    // Verify no error message appears in the PDF viewer
    const errorText = page.locator(".text-destructive").filter({ hasText: /load failed|error/i });
    await expect(errorText).toHaveCount(0);
  });

  test("inserts an internal preview link into the editor and reopens the preview from the message", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("preview-link.md", "# preview link")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("preview-link.md");

    await actor.attemptsTo(
      PreviewRoomFile("preview-link.md"),
      InsertPreviewRoomFileMarkdown(),
    );

    await expect(RoomScreen.filePreviewDialog(page)).toHaveCount(0);
    await RoomScreen.sendButton(page).click();

    const lastMessage = RoomScreen.messageContents(page).last();
    const link = lastMessage.locator('a[href^="/contents/"]');

    await expect(link).toBeVisible();
    await expect(lastMessage.locator('a[href*="/api/v1/contents/"]')).toHaveCount(0);

    const currentUrl = page.url();
    await link.click();

    await expect(page).toHaveURL(currentUrl);
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
  });

  test("deletes uploaded files without leaving stale entries behind", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      UploadRoomFiles(textFile("to-delete.txt", "delete me")),
    );

    await expect.poll(async () => actor.answer(FileCount())).toBe(1);

    await actor.attemptsTo(
      DeleteRoomFile("to-delete.txt"),
    );

    await expect.poll(async () => actor.answer(FileCount())).toBe(0);
    await expect(RoomScreen.fileEmptyState(page)).toBeVisible();
  });

  test("uploads files via chunked upload and downloads successfully", async ({
    actor,
    page,
  }) => {
    // Set a very low threshold (10 bytes) to force chunked upload
    await page.evaluate(() => {
      (window as any).__CHUNKED_UPLOAD_THRESHOLD = 10;
    });

    await actor.attemptsTo(
      UploadRoomFiles(
        textFile("chunked-file.txt", "This is a file that is long enough to exceed the 10-byte threshold for chunked uploads."),
      ),
    );

    await expect(RoomScreen.fileEmptyState(page)).toHaveCount(0);
    await expect.poll(async () => actor.answer(FileCount())).toBe(1);
    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("chunked-file.txt");

    // Verify it can be previewed and downloaded successfully
    await actor.attemptsTo(
      PreviewRoomFile("chunked-file.txt"),
    );

    const downloadPromise = page.waitForEvent("download");
    await actor.attemptsTo(
      DownloadPreviewedRoomFile(),
    );
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toBe("chunked-file.txt");

    const filePath = await download.path();
    expect(filePath).toBeTruthy();
  });
});
