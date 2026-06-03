import type { BrowserContext, Page } from "@playwright/test";

type ScriptTarget = BrowserContext | Page;

declare global {
  interface Window {
    __e2eClipboard?: string;
  }
}

export async function installClipboardStub(target: ScriptTarget): Promise<void> {
  await target.addInitScript(() => {
    window.__e2eClipboard = "";

    const clipboard = {
      writeText: async (text: string) => {
        window.__e2eClipboard = String(text);
      },
      readText: async () => window.__e2eClipboard ?? "",
    };

    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: clipboard,
    });
  });
}

export async function readClipboard(page: Page): Promise<string> {
  return page.evaluate(() => window.__e2eClipboard ?? "");
}
