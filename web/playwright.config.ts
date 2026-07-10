import { defineConfig, devices } from "@playwright/test";
import type {
  SerenityFixtures,
  SerenityWorkerFixtures,
} from "@serenity-js/playwright-test";

// 强制本地调试绕过系统代理，避免 localhost 被代理导致 502
process.env.NO_PROXY = "localhost,127.0.0.1,::1";
delete process.env.http_proxy;
delete process.env.https_proxy;
delete process.env.NO_COLOR;

const appBaseUrl = process.env.PLAYWRIGHT_BASE_URL ??
  "http://127.0.0.1:4093";
const serverPort = new URL(appBaseUrl).port || "4093";
const runId = process.env.PLAYWRIGHT_RUN_ID ?? String(process.pid);
const databaseUrl = `sqlite://target/playwright-${runId}.db?mode=rwc`;
const storageRoot = `target/playwright-storage-${runId}`;

/**
 * See https://playwright.dev/docs/test-configuration.
 *
 * 单端口架构：Rust 后端同时 serve API 和嵌入的 SPA 静态文件
 * - 默认使用独立测试端口 4093，可通过 PLAYWRIGHT_BASE_URL 覆盖
 * - /api/v1/* → Axum 后端路由
 * - /* → rust-embed 嵌入的 Next.js 静态文件（SPA fallback）
 */
export default defineConfig<SerenityFixtures, SerenityWorkerFixtures>({
  testDir: "./e2e/specs",
  /* Run tests in files in parallel */
  fullyParallel: false,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [
    ["line"],
    ["html", { open: "never" }],
    ["@serenity-js/playwright-test", {
      crew: [
        "@serenity-js/console-reporter",
        ["@serenity-js/serenity-bdd", {
          specDirectory: "./e2e/specs",
          reporter: {
            includeAbilityDetails: true,
          },
        }],
        ["@serenity-js/core:ArtifactArchiver", {
          outputDirectory: "target/site/serenity",
        }],
      ],
    }],
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* 单端口：前端和后端使用同一个隔离的测试地址 */
    baseURL: appBaseUrl,
    defaultActorName: "Alice",
    crew: [
      ["@serenity-js/web:Photographer", {
        strategy: "TakePhotosOfFailures",
      }],
    ],
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    /* 增加超时时间 */
    navigationTimeout: 60000,
    actionTimeout: 30000,
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        /* 禁用沙箱以避免浏览器崩溃 */
        launchOptions: {
          args: ["--no-sandbox", "--disable-setuid-sandbox"],
        },
      },
    },
  ],

  /* 全局超时 */
  timeout: 120 * 1000,
  expect: {
    timeout: 30 * 1000,
  },

  /* 每次测试使用独立 DB/storage；仅显式设置时复用外部服务器。 */
  webServer: {
    command:
      `PORT=${serverPort} DATABASE_URL='${databaseUrl}' ` +
      `DB_MAX_CONNECTIONS=5 DB_MIN_CONNECTIONS=1 ` +
      `ELIZABETH__APP__STORAGE__ROOT='${storageRoot}' ` +
      `cargo run -p elizabeth-board -- run`,
    url: `${appBaseUrl}/api/v1/health`,
    reuseExistingServer: process.env.PLAYWRIGHT_REUSE_SERVER === "1",
    timeout: 180 * 1000,
    cwd: "..",
  },
});
