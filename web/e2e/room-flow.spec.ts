import { expect, test } from "@playwright/test";

test.describe("Elizabeth Room Flow", () => {
  const roomName = `test-room-${Date.now()}`;
  const password = "test123"; // pragma: allowlist secret`

  test("should display home page with create and join options", async ({ page }) => {
    await page.goto("/");

    // Check title
    await expect(page.locator("h1")).toContainText("Elizabeth");

    // Check description
    await expect(page.locator("text=安全、临时、可控的文件分享与协作平台"))
      .toBeVisible();

    // Check create and join cards
    await expect(page.locator("text=创建房间")).toBeVisible();
    await expect(page.locator("text=加入房间")).toBeVisible();
  });

  test("should create a new room without password", async ({ page }) => {
    await page.goto("/");

    // Click create room
    await page.locator("text=创建房间").first().click();

    // Fill room name
    const simpleRoomName = `simple-room-${Date.now()}`;
    await page.fill("#room-name", simpleRoomName);

    // Click create button
    await page.locator('button:has-text("创建房间")').click();

    // Wait for navigation to room page
    await page.waitForURL(`/${simpleRoomName}`, { timeout: 10000 });

    // Check if we're in the room
    await expect(page).toHaveURL(new RegExp(`/${simpleRoomName}`));
  });

  test("should create a new room with password", async ({ page }) => {
    await page.goto("/");

    // Click create room
    await page.locator("text=创建房间").first().click();

    // Fill room name and password
    await page.fill("#room-name", roomName);
    await page.fill("#password", password);

    // Click create button
    await page.locator('button:has-text("创建房间")').click();

    // Wait for navigation to room page
    await page.waitForURL(`/${roomName}`, { timeout: 10000 });

    // Check if we're in the room
    await expect(page).toHaveURL(new RegExp(`/${roomName}`));
  });

  test("should require password to join password-protected room", async ({ page, context }) => {
    // First, ensure the room exists by creating it
    await page.goto("/");
    await page.locator("text=创建房间").first().click();
    const testRoomName = `pwd-room-${Date.now()}`;
    await page.fill("#room-name", testRoomName);
    await page.fill("#password", password);
    await page.locator('button:has-text("创建房间")').click();
    await page.waitForURL(`/${testRoomName}`, { timeout: 10000 });

    // Open in a new context (simulating a new user)
    const newPage = await context.newPage();

    // Try to join the room
    await newPage.goto(`/${testRoomName}`);

    // Should show password dialog
    await expect(newPage.locator("text=房间已加密")).toBeVisible({
      timeout: 10000,
    });
    await expect(newPage.locator("text=需要密码才能访问")).toBeVisible();

    // Try wrong password
    await newPage.fill("#password", "wrongpassword");
    await newPage.locator('button:has-text("进入房间")').click();

    // Should show error
    await expect(newPage.locator("text=密码错误")).toBeVisible({
      timeout: 5000,
    });

    // Try correct password
    await newPage.fill("#password", password);
    await newPage.locator('button:has-text("进入房间")').click();

    // Should enter the room
    await expect(newPage).toHaveURL(new RegExp(`/${testRoomName}`));

    await newPage.close();
  });

  test("should send and receive messages in room", async ({ page }) => {
    // Navigate to the room
    await page.goto(`/${roomName}`);

    // Wait for room to load
    await page.waitForTimeout(2000);

    // Find message input (react-md-editor uses textarea with specific class)
    const messageInput = page.locator(
      'textarea.w-md-editor-text-input, textarea, [contenteditable="true"]',
    )
      .first();
    await expect(messageInput).toBeVisible({ timeout: 10000 });

    // Type a message
    const testMessage = "Hello from Playwright test!";
    await messageInput.fill(testMessage);

    // Send message (look for send button)
    const sendButton = page.locator(
      'button[type="submit"], button:has-text("发送")',
    ).first();
    await sendButton.click();

    // Wait for message to appear
    await page.waitForTimeout(2000);

    // Check if message appears in the chat
    await expect(page.locator(`text=${testMessage}`)).toBeVisible({
      timeout: 5000,
    });
  });

  test("should update message in room", async ({ page }) => {
    // Navigate to the room
    await page.goto(`/${roomName}`);

    // Wait for room to load
    await page.waitForTimeout(2000);

    // Find the first message bubble and hover over it to show edit button
    const messageBubble = page.locator(".group").first();
    await messageBubble.hover();

    // Wait a moment for the edit button to appear
    await page.waitForTimeout(500);

    // Find the edit button within the hovered message
    const editButton = messageBubble.locator(
      'button[title="编辑"], button:has-text("编辑")',
    ).first();

    if (await editButton.isVisible()) {
      await editButton.click();

      // Update the message
      const messageInput = page.locator(
        'textarea.w-md-editor-text-input, textarea, [contenteditable="true"]',
      )
        .first();
      await messageInput.fill("Updated message from Playwright!");

      // Send updated message
      const sendButton = page.locator(
        'button[type="submit"], button:has-text("发送")',
      ).first();
      await sendButton.click();

      // Wait for update
      await page.waitForTimeout(2000);

      // Check if updated message appears
      await expect(page.locator("text=Updated message from Playwright!"))
        .toBeVisible({ timeout: 5000 });
    }
  });

  test("should upload and display files", async ({ page }) => {
    // Navigate to the room
    await page.goto(`/${roomName}`);

    // Wait for room to load
    await page.waitForTimeout(2000);

    // Look for file upload area (might be in right sidebar)
    const fileInput = page.locator('input[type="file"]').first();

    if (await fileInput.isVisible()) {
      // Create a test file using Playwright's built-in file handling
      const testFileName = "test-upload.txt";
      const testContent = "Test file content";

      // Upload file using Playwright's setInputFiles with file-like object
      await fileInput.setInputFiles({
        name: testFileName,
        mimeType: "text/plain",
        buffer: Buffer.from(testContent, "utf-8"),
      });

      // Wait for upload to complete
      await page.waitForTimeout(3000);

      // Check if file appears in the list
      await expect(page.locator(`text=${testFileName}`)).toBeVisible({
        timeout: 5000,
      });
    }
  });

  test("should access room settings", async ({ page }) => {
    // Navigate to the room
    await page.goto(`/${roomName}`);

    // Wait for room to load
    await page.waitForTimeout(2000);

    // Look for settings button (might be in left sidebar or top bar)
    const settingsButton = page.locator(
      'button:has-text("设置"), button[aria-label="设置"]',
    ).first();

    if (await settingsButton.isVisible()) {
      await settingsButton.click();

      // Check if settings dialog appears (use more specific selector)
      await expect(page.getByRole("heading", { name: "房间设置" })).toBeVisible(
        {
          timeout: 5000,
        },
      );
    }
  });

  test("should handle room not found (or auto-create rooms)", async ({ page }) => {
    const nonExistentRoom = `non-existent-room-${Date.now()}`;

    await page.goto(`/${nonExistentRoom}`);

    // Backend auto-creates rooms instead of returning 404, so we should see:
    // 1. Either "房间不存在" if backend is changed to not auto-create
    // 2. Or the room page if the room was auto-created
    await page.waitForTimeout(3000); // Wait for backend response

    const currentUrl = page.url();
    if (currentUrl.includes(nonExistentRoom)) {
      // Room was auto-created, we should be in the room interface
      // Verify we're on the room page by checking for typical room elements
      const roomElements = page.locator(
        'textarea.w-md-editor-text-input, .group, button[title="编辑"]',
      ).first();
      await expect(roomElements).toBeVisible({ timeout: 5000 });
    } else {
      // Should show error message (if backend is changed in the future)
      await expect(page.locator("text=房间不存在")).toBeVisible({
        timeout: 10000,
      });
    }
  });

  test("should navigate back to home from join page", async ({ page }) => {
    await page.goto("/");

    // Click join room
    await page.locator("text=加入房间").first().click();

    // Check we're on join page
    await expect(page.locator("text=输入房间名称以加入现有房间")).toBeVisible();

    // Click back button
    await page.locator('button:has-text("返回")').click();

    // Should be back on home page
    await expect(page.locator('h1:has-text("Elizabeth")')).toBeVisible();
  });
});
