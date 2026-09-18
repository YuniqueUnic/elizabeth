import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import {
  ADMIN_BOOTSTRAP_PASSWORD,
  ADMIN_BOOTSTRAP_USERNAME,
} from "../../screenplay/abilities/CallElizabethApi.ability";
import { tAdmin, tCommon, tRoom } from "../../screenplay/support/i18n";
import { parseUtcMillis, roomExpiryPolicy } from "../../screenplay/support/room-expiry";
import { uniqueRoomName } from "../../screenplay/support/test-data";

/**
 * 平台管理面板 E2E：
 * 覆盖 happy path 与异常路径——错误凭证、搜索空态、删除确认/取消、
 * 会话持久化、退出登录、改密、运行时配置写入与越界拒绝。
 */

const SESSION_KEY = "elizabeth.admin-session";

/** 经 API 登录平台管理员，返回 Bearer 会话头（服务端鉴权以服务端为准）。 */
async function adminAuthHeaders(
  request: import("@playwright/test").APIRequestContext,
): Promise<Record<string, string>> {
  const response = await request.post("/api/v1/admin/auth/login", {
    data: { username: ADMIN_BOOTSTRAP_USERNAME, password: ADMIN_BOOTSTRAP_PASSWORD },
  });
  expect(response.ok()).toBeTruthy();
  const body = await response.json();
  return { Authorization: `Bearer ${body.token}` };
}

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

async function login(
  page: import("@playwright/test").Page,
  username: string,
  password: string,
): Promise<void> {
  await page.getByLabel(tAdmin("login.usernameLabel")).fill(username);
  await page.getByLabel(tAdmin("login.passwordLabel")).fill(password);
  await page.getByRole("button", { name: tAdmin("login.submit") }).click();
}

test.describe("Admin panel", () => {
  test("rejects invalid credentials and stays on the login screen", async ({ page }) => {
    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, "definitely-wrong-password");

    await expect(page.getByText(tAdmin("login.invalid")).first()).toBeVisible();
    await expect(page.getByLabel(tAdmin("login.usernameLabel"))).toBeVisible();
    // 失败的登录不得留下任何会话痕迹
    expect(await page.evaluate((key) => sessionStorage.getItem(key), SESSION_KEY)).toBeNull();
  });

  test("logs in, shows dashboard stats, and persists the session across reload", async ({
    page,
  }) => {
    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);

    // 概览统计卡片可见
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();
    await expect(page.getByText(tAdmin("stats.logicalBytes"))).toBeVisible();

    // 会话持久化：刷新后仍在面板内（不再出现登录表单）
    await page.reload();
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();

    // 退出登录 → 回到登录页并清空会话
    await page.getByRole("button", { name: tAdmin("nav.logout") }).click();
    await expect(page.getByLabel(tAdmin("login.usernameLabel"))).toBeVisible();
    expect(await page.evaluate((key) => sessionStorage.getItem(key), SESSION_KEY)).toBeNull();
  });

  test("manages rooms: search, empty state, detail, delete cancel and confirm", async ({
    page,
    request,
  }) => {
    const roomName = uniqueRoomName("admin-panel");
    await createRoomViaApi(request, roomName);

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);
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
      headers: await adminAuthHeaders(request),
    });
    expect(detail.status()).toBe(404);
  });

  test("updates runtime config from the system tab and validates input ranges", async ({
    page,
    request,
  }) => {
    // 运行时覆盖存活于服务进程内：先归零，保证断言的初始状态与执行顺序无关
    const reset = await request.put("/api/v1/admin/config/runtime", {
      headers: await adminAuthHeaders(request),
      data: {
        disallow_search_indexing: true,
        room_default_max_size: 0,
        room_default_max_times_entered: 0,
        room_expiry: { allowed_ages_seconds: [], default_age_seconds: 0 },
      },
    });
    expect(reset.ok()).toBeTruthy();
    const { defaultAge: deploymentDefaultAge } = await roomExpiryPolicy(request);

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);
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
      headers: await adminAuthHeaders(request),
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

    // 房间有效期策略：允许列表与默认时长是一组，写入后同时生效
    await page.getByTestId("admin-runtime-expiry-ages").fill("5m, 30m");
    await page.getByTestId("admin-runtime-expiry-default").fill("30m");
    await page.getByTestId("admin-save-runtime").click();
    await expect(page.getByText(tAdmin("system.saved"), { exact: true }).first()).toBeVisible();

    // 新建房间继承覆盖后的默认时长（30m）
    const overrideRoom = uniqueRoomName("admin-expiry");
    await createRoomViaApi(request, overrideRoom);
    const overrideDetail = await request.get(
      `/api/v1/admin/rooms/${encodeURIComponent(overrideRoom)}`,
      { headers: await adminAuthHeaders(request) },
    );
    expect(overrideDetail.ok()).toBeTruthy();
    expect(
      Math.abs(
        (parseUtcMillis((await overrideDetail.json()).expire_at) - Date.now()) / 1000 - 1800,
      ),
    ).toBeLessThan(30);

    // 覆盖是整组替换：部署默认时长已不在允许列表内，建房被拒且不落库
    const rejectedRoom = uniqueRoomName("admin-expiry-rejected");
    const rejected = await request.post(`/api/v1/rooms/${encodeURIComponent(rejectedRoom)}`, {
      data: { age_seconds: deploymentDefaultAge },
    });
    expect(rejected.status()).toBe(400);
    const rejectedDetail = await request.get(
      `/api/v1/admin/rooms/${encodeURIComponent(rejectedRoom)}`,
      { headers: await adminAuthHeaders(request) },
    );
    expect(rejectedDetail.status()).toBe(404);

    // 清除覆盖 → 回退配置文件策略，部署默认时长重新可用
    await page.getByTestId("admin-clear-room-expiry").click();
    await expect(page.getByTestId("admin-room-expiry-base")).toBeVisible();
    const restoredRoom = uniqueRoomName("admin-expiry-restored");
    await createRoomViaApi(request, restoredRoom);
    const restoredDetail = await request.get(
      `/api/v1/admin/rooms/${encodeURIComponent(restoredRoom)}`,
      { headers: await adminAuthHeaders(request) },
    );
    expect(restoredDetail.ok()).toBeTruthy();
    expect(
      Math.abs(
        (parseUtcMillis((await restoredDetail.json()).expire_at) - Date.now()) / 1000 -
          deploymentDefaultAge,
      ),
    ).toBeLessThan(30);
  });

  test("lists existing rooms without search, edits settings, mints a code", async ({
    page,
    request,
  }) => {
    const roomName = uniqueRoomName("admin-list");
    await createRoomViaApi(request, roomName);
    const { allowedAges } = await roomExpiryPolicy(request);
    const longestAge = allowedAges[allowedAges.length - 1];

    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);
    await page.getByRole("button", { name: tAdmin("nav.rooms") }).click();

    // 默认列出已有房间，无需先搜索
    await expect(page.getByText(roomName)).toBeVisible();

    // 行内「配置」按钮打开配置对话框 → 设置 tab 修改进入次数上限、房间持续时间与上传类型策略
    const configRow = page.locator("div.border-t", { hasText: roomName }).first();
    await configRow.getByTestId("admin-room-configure").click();
    await page.getByRole("tab", { name: tAdmin("rooms.tabSettings") }).click();
    await page.getByLabel(tAdmin("rooms.maxTimes")).fill("5");
    await page.getByTestId("admin-room-duration").click();
    await page.getByTestId(`admin-room-duration-option-${longestAge}`).click();
    await page.getByTestId("admin-room-upload-file-type-mode").click();
    await page
      .getByRole("option", { name: tRoom("config.uploadFileType.mode.allow") })
      .click();
    await page.getByTestId("admin-room-upload-file-type-extensions").fill("pdf, zip");
    await page.getByTestId("admin-room-save").click();
    await expect(page.getByText(tAdmin("rooms.saved")).first()).toBeVisible();

    // 服务端确认三个字段都已生效：截止时刻按所选时长重算，上传策略已落盘
    const detail = await request.get(
      `/api/v1/admin/rooms/${encodeURIComponent(roomName)}`,
      { headers: await adminAuthHeaders(request) },
    );
    expect(detail.ok()).toBeTruthy();
    const saved = await detail.json();
    expect(saved.max_times_entered).toBe(5);
    expect(
      Math.abs((parseUtcMillis(saved.expire_at) - Date.now()) / 1000 - longestAge),
    ).toBeLessThan(30);
    expect(saved.upload_file_type).toEqual({
      mode: "allow",
      extensions: ["pdf", "zip"],
    });

    // 概览 tab 回显当前上传策略（与房间侧边栏同源）
    await page.getByRole("tab", { name: tAdmin("rooms.tabOverview") }).click();
    await expect(
      page.getByRole("dialog").locator("dd", {
        hasText: tRoom("config.uploadFileType.mode.allow"),
      }),
    ).toHaveText(`${tRoom("config.uploadFileType.mode.allow")} (pdf, zip)`);

    // 身份码 tab：铸造 editor 身份码，明文仅此一次展示，可复制
    await page.getByRole("tab", { name: tAdmin("rooms.tabIdentity") }).click();
    await page.getByTestId("admin-room-mint").click();
    const minted = page.getByTestId("minted-code");
    await expect(minted).toBeVisible();
    const code = await minted.locator("code").textContent();
    expect(code?.length).toBeGreaterThanOrEqual(6);
    await page.keyboard.press("Escape");
  });

  test("changes the admin password and re-logs in", async ({ page }) => {
    const newPassword = "rotated-e2e-password"; // pragma: allowlist secret
    await gotoAdmin(page);
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();

    // 改密成功后本会话立即失效，面板回到登录页
    await page.getByTestId("admin-password-current").fill(ADMIN_BOOTSTRAP_PASSWORD);
    await page.getByTestId("admin-password-new").fill(newPassword);
    await page.getByTestId("admin-password-confirm").fill(newPassword);
    await page.getByTestId("admin-change-password").click();
    await expect(page.getByText(tAdmin("system.passwordChanged")).first()).toBeVisible();
    await expect(page.getByLabel(tAdmin("login.usernameLabel"))).toBeVisible();

    // 旧密码失效，新密码可登录
    await login(page, ADMIN_BOOTSTRAP_USERNAME, ADMIN_BOOTSTRAP_PASSWORD);
    await expect(page.getByText(tAdmin("login.invalid")).first()).toBeVisible();
    await login(page, ADMIN_BOOTSTRAP_USERNAME, newPassword);
    // 登录后 tab 状态保留在系统 tab，切回概览验证统计加载
    await page.getByRole("button", { name: tAdmin("nav.dashboard") }).click();
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();

    // 改回原密码，保持环境对后续用例可用
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();
    await page.getByTestId("admin-password-current").fill(newPassword);
    await page.getByTestId("admin-password-new").fill(ADMIN_BOOTSTRAP_PASSWORD);
    await page.getByTestId("admin-password-confirm").fill(ADMIN_BOOTSTRAP_PASSWORD);
    await page.getByTestId("admin-change-password").click();
    await expect(page.getByLabel(tAdmin("login.usernameLabel"))).toBeVisible();
  });

  test("links to the panel from the home page entry", async ({ page }) => {
    await page.goto("/");
    const entry = page.getByRole("link", { name: tCommon("adminEntry") });
    await expect(entry).toBeVisible();
    await entry.click();
    await expect(page.getByLabel(tAdmin("login.usernameLabel"))).toBeVisible();
  });
});
