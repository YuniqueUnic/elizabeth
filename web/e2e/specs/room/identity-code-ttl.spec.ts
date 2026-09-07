import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { CallElizabethApi } from "../../screenplay/abilities/CallElizabethApi.ability";
import { OpenRoom } from "../../screenplay/room/tasks/Room.tasks";
import { API_BASE_URL } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";

/** naive datetime（无时区后缀）按 UTC 解析为毫秒时间戳。 */
function parseNaiveUtcMs(value: string): number {
  const normalized = value.replace(" ", "T");
  return new Date(normalized.endsWith("Z") ? normalized : `${normalized}Z`).getTime();
}

test.describe("Identity code lifetime semantics", () => {
  test("creator admin code follows the room lifetime", async ({ request }) => {
    const api = CallElizabethApi.using(request);
    const roomName = uniqueRoomName("screenplay-ttl-admin");
    const admin = await api.ensureRoom(roomName);

    const roomResponse = await request.get(`${API_BASE_URL}/rooms/${roomName}`);
    expect(roomResponse.ok()).toBeTruthy();
    const roomInfo = await roomResponse.json();

    const driftSeconds =
      Math.abs(parseNaiveUtcMs(admin.expiresAt) - parseNaiveUtcMs(roomInfo.expire_at)) / 1000;
    expect(driftSeconds).toBeLessThanOrEqual(6);
  });

  test("editor code honors admin-configured duration", async ({ request }) => {
    const api = CallElizabethApi.using(request);
    const roomName = uniqueRoomName("screenplay-ttl-editor");
    const admin = await api.ensureRoom(roomName);

    const ttlSeconds = 3600;
    const before = Date.now();
    const editor = await api.issueRoleToken(roomName, "editor", admin.token!, ttlSeconds);

    const expectedMs = before + ttlSeconds * 1000;
    const driftSeconds = Math.abs(parseNaiveUtcMs(editor.expiresAt) - expectedMs) / 1000;
    expect(driftSeconds).toBeLessThanOrEqual(10);
  });

  test("out-of-range durations are rejected", async ({ request }) => {
    const api = CallElizabethApi.using(request);
    const roomName = uniqueRoomName("screenplay-ttl-invalid");
    const admin = await api.ensureRoom(roomName);

    for (const invalid of [30, -5]) {
      const status = await api.tryIssueRoleToken(roomName, "editor", admin.token!, invalid);
      expect(status).toBe(400);
    }
  });

  test("member panel exposes the duration configuration", async ({ actor, page, provisionRoom }) => {
    const roomName = uniqueRoomName("screenplay-ttl-ui");
    const room = await provisionRoom({ actor, roomName });

    await actor.attemptsTo(OpenRoom(room.url));
    await page.getByRole("button", { name: /成员与权限|Members & permissions/ }).click();
    await expect(page.getByTestId("editor-token-duration-value")).toBeVisible();
    await expect(page.getByTestId("editor-token-duration-unit")).toBeVisible();
  });
});
