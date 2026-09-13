import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { ADMIN_BOOTSTRAP_TOKEN } from "../../screenplay/abilities/CallElizabethApi.ability";
import { tAdmin, tCommon } from "../../screenplay/support/i18n";
import { uniqueRoomName } from "../../screenplay/support/test-data";

/**
 * 平台管理面板 E2E（issue #196 第三阶段）：
 * 覆盖 happy path 与异常路径——错误 token、搜索空态、删除确认/取消、
 * 会话持久化、退出登录、运行时配置写入与越界拒绝。
 */

const adminHeaders = { "X-Elizabeth-Admin-Token": ADMIN_BOOTSTRAP_TOKEN };

async function createRoomViaApi(
  request: import("@playwright/test").APIRequestContext,
  name: string,
): Promise<void> {
  const response = await request.post(`/api/v1/rooms/${encodeURIComponent(name)}`, {
    data: {},
  });
  if (!response.ok() && response.status() !== 409) {
    throw new Error(`create room failed: ${response.status()}`);
  }
}

async function gotoAdmin(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/admin");
}

async function login(page: import("@playwright/test").Page, token: string): Promise<void> {
  await page.getByLabel(tAdmin("login.tokenLabel")).fill(token);
  await page.getByRole("button", { name: tAdmin("login.submit") }).click();
}

test.describe("Admin panel", () => {
  test("rejects an invalid token and stays on the login screen", async ({ page }) => {
    await gotoAdmin(page);
    await login(page, "definitely-wrong-token");

    await expect(page.getByText(tAdmin("login.invalid")).first()).toBeVisible();
    await expect(page.getByLabel(tAdmin("login.tokenLabel"))).toBeVisible();
    // 失败的登录不得留下任何会话痕迹
    expect(await page.evaluate(() => sessionStorage.getItem("elizabeth.admin-token"))).toBeNull();
  });

  test("logs in, shows dashboard stats, and persists the session across reload", async ({
    page,
  }) => {
    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_TOKEN);

    // 概览统计卡片可见
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();
    await expect(page.getByText(tAdmin("stats.logicalBytes"))).toBeVisible();

    // 会话持久化：刷新后仍在面板内（不再出现登录表单）
    await page.reload();
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();

    // 退出登录 → 回到登录页并清空会话
    await page.getByRole("button", { name: tAdmin("nav.logout") }).click();
    await expect(page.getByLabel(tAdmin("login.tokenLabel"))).toBeVisible();
    expect(await page.evaluate(() => sessionStorage.getItem("elizabeth.admin-token"))).toBeNull();
  });

  test("manages rooms: search, empty state, detail, delete cancel and confirm", async ({
    page,
    request,
  }) => {
    const roomName = uniqueRoomName("admin-panel");
    await createRoomViaApi(request, roomName);

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_TOKEN);
    await page.getByRole("button", { name: tAdmin("nav.rooms") }).click();

    // 精确搜索命中
    await page.getByLabel(tAdmin("rooms.searchPlaceholder")).fill(roomName);
    await page.getByRole("button", { name: tAdmin("rooms.search") }).click();
    await expect(page.getByText(roomName)).toBeVisible();

    // 无匹配 → 空态
    await page.getByLabel(tAdmin("rooms.searchPlaceholder")).fill("no-such-room-xyz");
    await page.getByRole("button", { name: tAdmin("rooms.search") }).click();
    await expect(page.getByText(tAdmin("rooms.empty"))).toBeVisible();

    // 回到目标房间并打开详情
    await page.getByLabel(tAdmin("rooms.searchPlaceholder")).fill(roomName);
    await page.getByRole("button", { name: tAdmin("rooms.search") }).click();
    const configRow = page.locator("div.border-t", { hasText: roomName }).first();
    await configRow.getByTestId("admin-room-configure").click();
    await expect(page.getByText(tAdmin("rooms.detailTokens"))).toBeVisible();
    await page.keyboard.press("Escape");

    // 删除取消：房间保留
    const row = page.locator("div.border-t", { hasText: roomName }).first();
    await row.getByRole("button", { name: tAdmin("rooms.delete") }).click();
    await expect(page.getByText(tAdmin("rooms.deleteConfirmTitle"))).toBeVisible();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: tAdmin("rooms.cancel") })
      .click();
    await expect(page.getByText(roomName)).toBeVisible();

    // 删除确认：房间从列表消失
    await row.getByRole("button", { name: tAdmin("rooms.delete") }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: tAdmin("rooms.delete") })
      .click();
    await expect(page.getByText(tAdmin("rooms.empty"))).toBeVisible();

    // 服务端确认已删除
    const detail = await request.get(`/api/v1/admin/rooms/${encodeURIComponent(roomName)}`, {
      headers: adminHeaders,
    });
    expect(detail.status()).toBe(404);
  });

  test("updates runtime config from the system tab and validates input ranges", async ({
    page,
    request,
  }) => {
    // 运行时覆盖存活于服务进程内：先归零，保证断言的初始状态与执行顺序无关
    const reset = await request.put("/api/v1/admin/config/runtime", {
      headers: adminHeaders,
      data: {
        disallow_search_indexing: true,
        room_default_max_size: 0,
        room_default_max_times_entered: 0,
      },
    });
    expect(reset.ok()).toBeTruthy();

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_TOKEN);
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();

    // robots 开关 → 服务端 robots.txt 即时生效（初始为禁止抓取）
    await page.getByTestId("admin-toggle-search-indexing").click();
    let robots = await request.get("/robots.txt");
    expect(await robots.text()).toContain("Allow: /");

    await page.getByTestId("admin-toggle-search-indexing").click();
    robots = await request.get("/robots.txt");
    expect(await robots.text()).toContain("Disallow: /");

    // 越界容量 → 服务端 400，错误 toast
    const maxSizeInput = page.getByLabel(tAdmin("system.runtimeMaxSize"));
    await maxSizeInput.fill("1");
    await page.getByTestId("admin-save-runtime").click();
    // 服务端返回原始校验消息，面板透传展示；toast 同时渲染可见 div 与 aria-live 通告，取首个
    await expect(page.getByText(/加载失败：Validation error/).first()).toBeVisible();

    // 合法容量 + 进入次数 → 保存成功提示
    await maxSizeInput.fill("1048576");
    await page.getByLabel(tAdmin("system.runtimeMaxTimes")).fill("7");
    await page.getByTestId("admin-save-runtime").click();
    await expect(page.getByText(tAdmin("system.saved"), { exact: true }).first()).toBeVisible();

    // 新房间确实继承覆盖值
    const roomName = uniqueRoomName("admin-runtime");
    await createRoomViaApi(request, roomName);
    const detail = await request.get(`/api/v1/admin/rooms/${encodeURIComponent(roomName)}`, {
      headers: adminHeaders,
    });
    expect(detail.ok()).toBeTruthy();
    const detailJson = await detail.json();
    expect(detailJson.max_size).toBe(1048576);
    expect(detailJson.max_times_entered).toBe(7);

    // 清除覆盖（传 0）
    await page.getByLabel(tAdmin("system.runtimeMaxSize")).fill("0");
    await page.getByLabel(tAdmin("system.runtimeMaxTimes")).fill("0");
    await page.getByTestId("admin-save-runtime").click();
    await expect(page.getByText(tAdmin("system.saved"), { exact: true }).first()).toBeVisible();
  });

  test("lists existing rooms without search, edits settings, mints a code", async ({
    page,
    request,
  }) => {
    const roomName = uniqueRoomName("admin-list");
    await createRoomViaApi(request, roomName);

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_TOKEN);
    await page.getByRole("button", { name: tAdmin("nav.rooms") }).click();

    // 默认列出已有房间，无需先搜索
    await expect(page.getByText(roomName)).toBeVisible();

    // 行内「配置」按钮打开配置对话框 → 设置 tab 修改进入次数上限
    const configRow = page.locator("div.border-t", { hasText: roomName }).first();
    await configRow.getByTestId("admin-room-configure").click();
    await page.getByRole("tab", { name: tAdmin("rooms.tabSettings") }).click();
    await page.getByLabel(tAdmin("rooms.maxTimes")).fill("5");
    await page.getByTestId("admin-room-save").click();
    await expect(page.getByText(tAdmin("rooms.saved")).first()).toBeVisible();

    // 身份码 tab：铸造 editor 身份码，明文仅此一次展示，可复制
    await page.getByRole("tab", { name: tAdmin("rooms.tabIdentity") }).click();
    await page.getByTestId("admin-room-mint").click();
    const minted = page.getByTestId("minted-code");
    await expect(minted).toBeVisible();
    const code = await minted.locator("code").textContent();
    expect(code?.length).toBeGreaterThanOrEqual(6);
    await page.keyboard.press("Escape");
  });

  test("rotates the admin credential and re-logs in", async ({ page }) => {
    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_TOKEN);
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();

    // 轮换后本会话自动切换凭证，来源标记为运行时覆盖
    await page.getByTestId("admin-credential-new").fill("rotated-e2e-admin-token");
    await page.getByTestId("admin-rotate-credential").click();
    await expect(page.getByTestId("admin-credential-source")).toHaveText(
      "runtime-override",
    );

    // 旧凭证已失效，新凭证可登录（退出前处于系统 tab，重登后先切回概览）
    await page.getByRole("button", { name: tAdmin("nav.logout") }).click();
    await login(page, ADMIN_BOOTSTRAP_TOKEN);
    await expect(page.getByText(tAdmin("login.invalid")).first()).toBeVisible();
    await login(page, "rotated-e2e-admin-token");
    await page.getByRole("button", { name: tAdmin("nav.dashboard") }).click();
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();

    // 轮换回原凭证，保持环境对后续用例可用
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();
    await page.getByTestId("admin-credential-new").fill(ADMIN_BOOTSTRAP_TOKEN);
    await page.getByTestId("admin-rotate-credential").click();
    await expect(page.getByTestId("admin-credential-source")).toHaveText(
      "runtime-override",
    );
  });

  test("links to the panel from the home page entry", async ({ page }) => {
    await page.goto("/");
    const entry = page.getByRole("link", { name: tCommon("adminEntry") });
    await expect(entry).toBeVisible();
    await entry.click();
    await expect(page.getByLabel(tAdmin("login.tokenLabel"))).toBeVisible();
  });
});
