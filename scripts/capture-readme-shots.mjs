// 一次性素材脚本：为 README 抓取平台截图（admin/editor 双视角、中英双语）。
// 前置: bash dev-assets/prepare-readme-demo.sh && 已有 /tmp/eliz-shots-env.json
// 用法: cd web && node dev-assets/capture-readme-shots.mjs [baseUrl] [outDir]
import { chromium } from "playwright";
import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const BASE_URL = process.argv[2] ?? "http://127.0.0.1:4092";
const OUT_DIR = process.argv[3] ?? "dev-assets/readme-shots";
const ENV_PATH = process.env.ELIZ_SHOTS_ENV ?? "/tmp/eliz-shots-env.json";
const ROOMS = { zh: "readme-demo", en: "readme-demo-en" };
const SETTLE_MS = 1200;

const tokens = JSON.parse(readFileSync(ENV_PATH, "utf8"));
mkdirSync(OUT_DIR, { recursive: true });

const browser = await chromium.launch({ headless: true });

const scrollToTop = async (page) => {
  await page.evaluate(() => {
    const boxes = [...document.querySelectorAll("*")].filter((el) => {
      const style = getComputedStyle(el);
      return (
        (style.overflowY === "auto" || style.overflowY === "scroll") &&
        el.scrollHeight > el.clientHeight + 50 &&
        el.querySelector('[data-testid^="message-item-"]')
      );
    });
    for (const el of boxes) el.scrollTop = 0;
  });
};

for (const locale of ["zh", "en"]) {
  const room = ROOMS[locale];
  const { admin, editor } = tokens[locale];
  const suffix = locale;

  // ---------- admin 视角上下文 ----------
  const adminContext = await browser.newContext({
    viewport: { width: 1600, height: 1000 },
    deviceScaleFactor: 2,
    locale: locale === "zh" ? "zh-CN" : "en-US",
  });
  await adminContext.addInitScript(
    ({ room, admin, locale }) => {
      localStorage.setItem(
        "elizabeth_tokens",
        JSON.stringify({
          [room]: {
            token: admin.token,
            expiresAt: admin.expiresAt,
            refreshToken: admin.refreshToken,
            capabilities: admin.capabilities,
            roleKey: admin.roleKey,
          },
        }),
      );
      localStorage.setItem(
        "elizabeth-storage",
        JSON.stringify({ state: { locale }, version: 0 }),
      );
    },
    { room, admin, locale },
  );

  const page = await adminContext.newPage();

  // 1) 首页
  await page.goto(`${BASE_URL}/`, { waitUntil: "domcontentloaded" });
  await page.waitForLoadState("networkidle").catch(() => {});
  await page.waitForTimeout(SETTLE_MS);
  await page.screenshot({ path: join(OUT_DIR, `home-${suffix}.png`) });
  console.log("saved", `home-${suffix}.png`);

  // 2) 房间三栏全景（滚动到顶部展示首条 Markdown 消息，并悬停展示操作栏）
  await page.goto(`${BASE_URL}/${room}`, { waitUntil: "domcontentloaded" });
  await page.waitForLoadState("networkidle").catch(() => {});
  await page.waitForTimeout(SETTLE_MS * 2);
  await scrollToTop(page);
  await page.waitForTimeout(400);
  const secondMessage = page.locator('[data-testid^="message-item-"]').nth(1);
  await secondMessage.hover().catch(() => {});
  await page.waitForTimeout(500);
  await page.screenshot({ path: join(OUT_DIR, `room-${suffix}.png`) });
  console.log("saved", `room-${suffix}.png`);

  // 3) 成员与权限弹窗 — 成员身份码 tab（默认）
  const manageButton = page.getByRole("button", {
    name: locale === "zh" ? "成员与权限" : "Members & permissions",
  });
  await manageButton.click();
  await page
    .getByRole("tab", {
      name: locale === "zh" ? "成员身份码" : "Identity codes",
    })
    .waitFor({ state: "visible", timeout: 10_000 });
  await page.waitForTimeout(SETTLE_MS);
  await page.screenshot({
    path: join(OUT_DIR, `permissions-members-${suffix}.png`),
  });
  console.log("saved", `permissions-members-${suffix}.png`);

  // 4) 成员与权限弹窗 — 角色权限 tab
  await page
    .getByRole("tab", {
      name: locale === "zh" ? "角色权限" : "Role permissions",
    })
    .click();
  await page.waitForTimeout(SETTLE_MS * 2);
  await page.screenshot({
    path: join(OUT_DIR, `permissions-roles-${suffix}.png`),
  });
  console.log("saved", `permissions-roles-${suffix}.png`);
  await adminContext.close();

  // ---------- editor 视角上下文 ----------
  const editorContext = await browser.newContext({
    viewport: { width: 1600, height: 1000 },
    deviceScaleFactor: 2,
    locale: locale === "zh" ? "zh-CN" : "en-US",
  });
  await editorContext.addInitScript(
    ({ room, editor, locale }) => {
      localStorage.setItem(
        "elizabeth_tokens",
        JSON.stringify({
          [room]: {
            token: editor.token,
            expiresAt: editor.expiresAt,
            refreshToken: editor.refreshToken,
            capabilities: editor.capabilities,
            roleKey: editor.roleKey,
          },
        }),
      );
      localStorage.setItem(
        "elizabeth-storage",
        JSON.stringify({ state: { locale }, version: 0 }),
      );
    },
    { room, editor, locale },
  );
  const editorPage = await editorContext.newPage();
  await editorPage.goto(`${BASE_URL}/${room}`, { waitUntil: "domcontentloaded" });
  await editorPage.waitForLoadState("networkidle").catch(() => {});
  await editorPage.waitForTimeout(SETTLE_MS * 2);
  await scrollToTop(editorPage);
  await editorPage.waitForTimeout(400);
  await editorPage
    .locator('[data-testid^="message-item-"]')
    .nth(1)
    .hover()
    .catch(() => {});
  await editorPage.waitForTimeout(500);
  await editorPage.screenshot({ path: join(OUT_DIR, `room-editor-${suffix}.png`) });
  console.log("saved", `room-editor-${suffix}.png`);
  await editorContext.close();
}

await browser.close();
console.log("all screenshots saved to", OUT_DIR);
