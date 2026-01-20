import { expect, test } from "@playwright/test";
import { RoomPage } from "../page-objects/room-page";

const BASE_URL = "http://localhost:4001";
const API_BASE_URL = process.env.API_BASE_URL || "http://localhost:4092/api/v1";
const TOKEN_STORAGE_KEY = "elizabeth_tokens";

async function ensureRoomExists(roomName: string) {
  const response = await fetch(
    `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}`,
    { method: "POST" },
  );

  if (!response.ok && response.status !== 409) {
    throw new Error(
      `Failed to create room ${roomName}: ${response.status} ${response.statusText}`,
    );
  }
}

async function issueRoomToken(roomName: string) {
  const response = await fetch(
    `${API_BASE_URL}/rooms/${encodeURIComponent(roomName)}/tokens`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    },
  );

  if (!response.ok) {
    const errorBody = await response.text().catch(() => "");
    throw new Error(
      `Failed to issue token for ${roomName}: ${response.status} ${response.statusText} ${errorBody}`,
    );
  }

  const token = await response.json();
  return {
    token: token.token as string,
    expiresAt: token.expires_at as string,
  };
}

test.describe("消息系统 - 多端实时同步", () => {
  test("A 端保存后，B 端应实时看到消息内容", async ({ browser }) => {
    const roomName = `playwright-ws-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
    const roomUrl = `${BASE_URL}/${roomName}`;

    await ensureRoomExists(roomName);
    const tokenA = await issueRoomToken(roomName);
    const tokenB = await issueRoomToken(roomName);

    const contextA = await browser.newContext();
    const contextB = await browser.newContext();

    await contextA.addInitScript(
      ({ storageKey, roomName, tokenInfo }) => {
        const existing = JSON.parse(
          window.localStorage.getItem(storageKey) || "{}",
        );
        existing[roomName] = tokenInfo;
        window.localStorage.setItem(storageKey, JSON.stringify(existing));
      },
      { storageKey: TOKEN_STORAGE_KEY, roomName, tokenInfo: tokenA },
    );

    await contextB.addInitScript(
      ({ storageKey, roomName, tokenInfo }) => {
        const existing = JSON.parse(
          window.localStorage.getItem(storageKey) || "{}",
        );
        existing[roomName] = tokenInfo;
        window.localStorage.setItem(storageKey, JSON.stringify(existing));
      },
      { storageKey: TOKEN_STORAGE_KEY, roomName, tokenInfo: tokenB },
    );

    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();

    const roomPageA = new RoomPage(pageA);
    const roomPageB = new RoomPage(pageB);

    await roomPageA.goto(roomUrl);
    await roomPageB.goto(roomUrl);

    await roomPageA.waitForRoomLoad();
    await roomPageB.waitForRoomLoad();

    const text = `hello-realtime-${Date.now()}`;
    await roomPageA.sendMessage(text);
    await roomPageA.topBar.saveBtn.click();

    await expect
      .poll(async () => await roomPageB.getLastMessageText(), { timeout: 10_000 })
      .toContain(text);

    await contextA.close();
    await contextB.close();
  });
});
