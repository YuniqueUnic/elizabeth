import type { BrowserContext, Page } from "@playwright/test";

type ScriptTarget = BrowserContext | Page;

export interface RecordedNotification {
  title: string;
  options?: NotificationOptions;
}

declare global {
  interface Window {
    __e2eNotifications?: RecordedNotification[];
    __e2eNotificationPermission?: NotificationPermission;
  }
}

export async function installNotificationStub(target: ScriptTarget): Promise<void> {
  await target.addInitScript(() => {
    window.__e2eNotifications = [];
    window.__e2eNotificationPermission = "default";

    class FakeNotification {
      static get permission(): NotificationPermission {
        return window.__e2eNotificationPermission ?? "default";
      }

      static async requestPermission(): Promise<NotificationPermission> {
        return window.__e2eNotificationPermission ?? "default";
      }

      title: string;
      options?: NotificationOptions;

      constructor(title: string, options?: NotificationOptions) {
        this.title = title;
        this.options = options;
        window.__e2eNotifications?.push({ title, options });
      }
    }

    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: true,
    });
    Object.defineProperty(window, "Notification", {
      configurable: true,
      value: FakeNotification,
    });
  });
}

export async function setNotificationPermission(
  page: Page,
  permission: NotificationPermission,
): Promise<void> {
  await page.evaluate((nextPermission) => {
    window.__e2eNotificationPermission = nextPermission;
  }, permission);
}

export async function readNotifications(page: Page): Promise<RecordedNotification[]> {
  return page.evaluate(() => window.__e2eNotifications ?? []);
}
