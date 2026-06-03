import type { BrowserContext, Page } from "@playwright/test";

import type { RoomTokenInfo } from "./constants";
import { TOKEN_STORAGE_KEY } from "./constants";

type ScriptTarget = BrowserContext | Page;

export async function primeRoomToken(
  target: ScriptTarget,
  roomName: string,
  tokenInfo: RoomTokenInfo,
): Promise<void> {
  await target.addInitScript(
    ({ roomName, tokenInfo, storageKey }) => {
      const existing = JSON.parse(
        window.localStorage.getItem(storageKey) || "{}",
      );
      existing[roomName] = tokenInfo;
      window.localStorage.setItem(storageKey, JSON.stringify(existing));
    },
    {
      roomName,
      tokenInfo,
      storageKey: TOKEN_STORAGE_KEY,
    },
  );
}
