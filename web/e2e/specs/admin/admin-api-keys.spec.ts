import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import {
  ADMIN_BOOTSTRAP_PASSWORD,
  ADMIN_BOOTSTRAP_USERNAME,
} from "../../screenplay/abilities/CallElizabethApi.ability";
import { tAdmin } from "../../screenplay/support/i18n";

/**
 * 管理 API key E2E：
 * 账号登录 → 在系统 tab 创建 API key（明文一次性展示）→ key 可调用管理 API →
 * 吊销后立即失效。机器凭证的生命周期全程走真实 UI 与真实服务端。
 */

const SESSION_KEY = "elizabeth.admin-session";

async function login(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/admin");
  await page.getByLabel(tAdmin("login.usernameLabel")).fill(ADMIN_BOOTSTRAP_USERNAME);
  await page.getByLabel(tAdmin("login.passwordLabel")).fill(ADMIN_BOOTSTRAP_PASSWORD);
  await page.getByRole("button", { name: tAdmin("login.submit") }).click();
  await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();
}

test.describe("Admin API keys", () => {
  test("creates a key through the UI, uses it, then revokes it", async ({
    page,
    request,
  }) => {
    await login(page);
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();

    // 创建：明文仅展示一次
    await page.getByTestId("admin-api-key-name").fill("e2e-ci");
    await page.getByTestId("admin-api-key-expiry").selectOption({ label: tAdmin("system.apiKeyExpiryWeek") });
    await page.getByTestId("admin-api-key-create").click();
    const secretBox = page.getByTestId("admin-api-key-secret");
    await expect(secretBox).toBeVisible();
    const secret = await secretBox.locator("code").textContent();
    expect(secret?.startsWith("elizabeth_ak_")).toBe(true);

    // key 能通过 X-Elizabeth-Admin-Key 调用管理 API（key 与登录会话互相独立）
    const stats = await request.get("/api/v1/admin/stats", {
      headers: { "X-Elizabeth-Admin-Key": secret! },
    });
    expect(stats.status()).toBe(200);
    const me = await request.get("/api/v1/admin/auth/me", {
      headers: { "X-Elizabeth-Admin-Key": secret! },
    });
    expect(me.status()).toBe(200);

    // 一次性展示语义：刷新后明文不再出现在页面任何位置，仅剩前缀条目
    await page.reload();
    await expect(page.getByText(tAdmin("stats.roomsTotal"))).toBeVisible();
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();
    await expect(page.getByText("e2e-ci").first()).toBeVisible();
    const bodyAfterReload = await page.locator("body").textContent();
    expect(bodyAfterReload).not.toContain(secret);

    // 吊销：UI 操作后 key 立即失效
    await page.getByTestId(/^admin-api-key-revoke-\d+$/).first().click();
    await expect(page.getByText(tAdmin("system.apiKeyEmpty"))).toBeVisible();
    const revoked = await request.get("/api/v1/admin/stats", {
      headers: { "X-Elizabeth-Admin-Key": secret! },
    });
    expect(revoked.status()).toBe(401);

    // 收尾：清除本会话
    await page.evaluate((key) => sessionStorage.removeItem(key), SESSION_KEY);
  });

  test("rejects key requests without credentials and empty names", async ({
    page,
    request,
  }) => {
    // 无凭证访问 key 列表 → 401
    const unauthenticated = await request.get("/api/v1/admin/api-keys");
    expect(unauthenticated.status()).toBe(401);

    await login(page);
    await page.getByRole("button", { name: tAdmin("nav.system") }).click();

    // 空白名称不允许提交（按钮禁用）；有效期下拉只提供合法选项，
    // 非法数值路径由服务端集成测试覆盖
    await expect(page.getByTestId("admin-api-key-create")).toBeDisabled();
    await page.getByTestId("admin-api-key-name").fill("   ");
    await expect(page.getByTestId("admin-api-key-create")).toBeDisabled();

    await page.evaluate((key) => sessionStorage.removeItem(key), SESSION_KEY);
  });
});
