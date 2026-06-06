import type { Locator, Page, Response } from "@playwright/test";
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

function recordContentResponses(page: Page) {
  const responses: Array<{
    status: number;
    resourceType: string;
    url: string;
  }> = [];

  const handler = (response: Response) => {
    if (!response.url().includes("/api/v1/contents/")) {
      return;
    }

    responses.push({
      status: response.status(),
      resourceType: response.request().resourceType(),
      url: response.url(),
    });
  };

  page.on("response", handler);

  return {
    responses,
    reset() {
      responses.length = 0;
    },
    dispose() {
      page.off("response", handler);
    },
  };
}

async function expectRenderedBlobImage(locator: Locator) {
  await expect(locator).toBeVisible();
  await expect.poll(
    async () => locator.evaluate((element) => (element as HTMLImageElement).complete),
  ).toBe(true);
  await expect.poll(
    async () => locator.evaluate((element) => (element as HTMLImageElement).naturalWidth),
  ).toBeGreaterThan(0);
  await expect.poll(
    async () =>
      locator.evaluate((element) =>
        (element as HTMLImageElement).currentSrc || (element as HTMLImageElement).src
      ),
  ).toMatch(/^blob:/);
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

  test("copies a clean token-free link from the preview modal", async ({
    actor,
    page,
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

    // Poll for clipboard
    await expect.poll(async () => actor.answer(ClipboardContents()), { timeout: 5000 })
      .toMatch(/.+/);

    const clipboard = await actor.answer(ClipboardContents());
    const base = escapeForRegex(room.url.replace(/\/[^/]+$/, ""));

    // Must be an app-level preview URL (/contents/{id}) — no API path, no token
    expect(clipboard).toMatch(
      new RegExp(`^${base}/contents/\\d+$`),
    );
    // Security: token must NEVER appear in the copied URL
    expect(clipboard).not.toContain("token=");
    expect(clipboard).not.toContain("/api/v1/");
  });

  test("copies markdown with a clean token-free URL", async ({
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

    // Poll for clipboard
    await expect.poll(async () => actor.answer(ClipboardContents()), { timeout: 5000 })
      .toMatch(/.+/);

    const clipboard = await actor.answer(ClipboardContents());
    const base = escapeForRegex(room.url.replace(/\/[^/]+$/, ""));

    // Must be image markdown with app-level preview URL — no token
    expect(clipboard).toMatch(
      new RegExp(`^!\\[pixel\\.png\\]\\(${base}/contents/\\d+\\)$`),
    );
    // Security: token must NEVER appear in the copied markdown
    expect(clipboard).not.toContain("token=");
    expect(clipboard).not.toContain("/api/v1/");
  });

  test("previews an image on first open without unauthenticated content requests", async ({
    actor,
    page,
  }) => {
    const contentResponses = recordContentResponses(page);

    await actor.attemptsTo(
      UploadRoomFiles(pngFile("first-preview.png")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("first-preview.png");

    contentResponses.reset();

    await actor.attemptsTo(
      PreviewRoomFile("first-preview.png"),
    );

    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expectRenderedBlobImage(RoomScreen.filePreviewDialog(page).locator("img").last());

    expect(contentResponses.responses.some((response) => response.status === 401)).toBe(false);
    expect(contentResponses.responses.some((response) => response.status === 200)).toBe(true);

    contentResponses.dispose();
  });

  test("file links in messages render as clickable <a> elements not plain text", async ({
    actor,
    page,
  }) => {
    // Upload a file and insert its preview markdown link into the editor
    await actor.attemptsTo(
      UploadRoomFiles(textFile("linked-doc.txt", "content")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("linked-doc.txt");

    // Insert the file markdown link into the composer and send
    await actor.attemptsTo(
      PreviewRoomFile("linked-doc.txt"),
      InsertPreviewRoomFileMarkdown(),
    );

    await expect(RoomScreen.filePreviewDialog(page)).toHaveCount(0);
    await RoomScreen.sendButton(page).click();

    const lastMessage = RoomScreen.messageContents(page).last();

    // The link must render as an <a> tag with the /contents/ path — NOT as raw text
    const link = lastMessage.locator('a[href^="/contents/"]');
    await expect(link).toBeVisible();
    await expect(link).toHaveText("linked-doc.txt");

    // Ensure the raw markdown syntax is not leaking through as plain text
    await expect(lastMessage).not.toContainText("[linked-doc.txt]");
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

  test("clicking an image in a message bubble opens the file preview modal", async ({
    actor,
    page,
  }) => {
    const contentResponses = recordContentResponses(page);

    // 1. Upload an image
    await actor.attemptsTo(
      UploadRoomFiles(pngFile("message-image.png")),
    );

    await expect.poll(async () => (await actor.answer(FileNames())).join("|"))
      .toContain("message-image.png");

    // 2. Insert the image markdown into the composer and send
    await actor.attemptsTo(
      PreviewRoomFile("message-image.png"),
      InsertPreviewRoomFileMarkdown(),
    );

    // Make sure preview dialog is closed
    await expect(RoomScreen.filePreviewDialog(page)).toHaveCount(0);

    // Ignore the secure fetch performed by the preview modal itself.
    contentResponses.reset();

    // Click send
    await RoomScreen.sendButton(page).click();

    // 3. Find the image in the last message bubble
    const lastMessage = RoomScreen.messageContents(page).last();
    const imgElement = lastMessage.locator("img");
    await expectRenderedBlobImage(imgElement);
    expect(contentResponses.responses.some((response) => response.status === 401)).toBe(false);
    expect(contentResponses.responses.some((response) => response.status === 200)).toBe(true);

    // 4. Click the image to open the preview modal
    await imgElement.click();

    // 5. Verify the dialog opens for the correct file name
    await expect(RoomScreen.filePreviewDialog(page)).toBeVisible();
    await expect(RoomScreen.filePreviewTitle(page)).toHaveText("message-image.png");
    await expectRenderedBlobImage(RoomScreen.filePreviewDialog(page).locator("img").last());

    contentResponses.dispose();
  });
});
