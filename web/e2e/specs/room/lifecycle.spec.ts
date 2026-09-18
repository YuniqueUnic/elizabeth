import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import { API_BASE_URL } from "../../screenplay/support/constants";
import {
  remainingSeconds,
  roomExpiryPolicy,
  roomView,
} from "../../screenplay/support/room-expiry";
import { tErrors } from "../../screenplay/support/i18n";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { ConfigureRoom, OpenRoom } from "../../screenplay/room/tasks/Room.tasks";

test.describe("Room lifecycle and limits", () => {
  test("rejects later visitors once the maximum view count has been reached", async ({
    actor,
    createActor,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-lifecycle"),
    });
    const visitorA = await createActor("Visitor-A");
    const visitorB = await createActor("Visitor-B");

    await actor.attemptsTo(
      OpenRoom(room.url),
      ConfigureRoom({ maxViews: 2 }),
    );

    await visitorA.actor.attemptsTo(OpenRoom(room.url));
    await expect(RoomScreen.messageInput(visitorA.page)).toBeVisible();

    await visitorB.page.goto(room.url);
    await expect(RoomScreen.alert(visitorB.page)).toBeVisible();
    await expect(RoomScreen.alert(visitorB.page)).toContainText(
      tErrors("roomInaccessibleViaLink"),
    );
  });

  // A 410 room is expired, not missing. Direct-URL provisioning must never
  // replace it; only a genuine repository miss may create a new room.
  test("blocks access after the room expires through the settings API without recreating it", async ({
    actor,
    createActor,
    page,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-expiry"),
    });

    await actor.attemptsTo(OpenRoom(room.url));

    const token = room.tokenInfo?.token;
    expect(token).toBeTruthy();
    if (!token) {
      throw new Error("Expected provisioned room token");
    }

    const updateResponse = await page.request.put(
      `${API_BASE_URL}/rooms/${room.name}/settings?token=${token}`,
      {
        data: {
          age_seconds: 60,
        },
        headers: {
          "Content-Type": "application/json",
        },
      },
    );
    expect(updateResponse.ok()).toBeTruthy();

    await expect.poll(
      async () => {
        const response = await page.request.get(
          `${API_BASE_URL}/rooms/${room.name}`,
        );
        return response.status();
      },
      {
        message: "room should become unavailable at its server-side deadline",
        timeout: 75_000,
        intervals: [1_000, 2_000, 5_000],
      },
    ).toBe(410);

    const roomResponse = await page.request.get(
      `${API_BASE_URL}/rooms/${room.name}`,
    );
    expect(roomResponse.status()).toBe(410);
    expect(await roomResponse.json()).toMatchObject({
      error: {
        code: "ROOM_EXPIRED",
        message: expect.stringMatching(/expired/i),
        status: 410,
      },
    });

    const tokenResponse = await page.request.post(
      `${API_BASE_URL}/rooms/${room.name}/tokens`,
      {
        data: { with_refresh_token: false },
      },
    );
    expect(tokenResponse.status()).toBe(410);
    expect(await tokenResponse.json()).toMatchObject({
      error: {
        code: "ROOM_EXPIRED",
        message: expect.stringMatching(/expired/i),
        status: 410,
      },
    });

    const visitor = await createActor("Expired-Visitor");
    let createRoomRequests = 0;
    visitor.page.on("request", (request) => {
      const path = new URL(request.url()).pathname;
      if (
        request.method() === "POST" &&
        path === `/api/v1/rooms/${room.name}`
      ) {
        createRoomRequests += 1;
      }
    });
    await visitor.page.goto(room.url);
    await expect(RoomScreen.alert(visitor.page)).toBeVisible();
    await expect(RoomScreen.alert(visitor.page)).toContainText(
      tErrors("roomExpired"),
    );
    expect(createRoomRequests).toBe(0);
  });

  // 侧边栏必须能改房间持续时间。新建房间的生命周期来自部署默认时长，
  // 缺少该控件时用户只能得到默认值（例如 2 小时）。
  test("applies the duration chosen in the sidebar to the room deadline", async ({
    actor,
    page,
    request,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-duration"),
    });
    await actor.attemptsTo(OpenRoom(room.url));

    const { allowedAges, defaultAge } = await roomExpiryPolicy(request);
    expect(allowedAges.length).toBeGreaterThan(1);

    // 新房间的寿命就是部署默认时长。
    const before = await remainingSeconds(request, room.name);
    expect(Math.abs(before - defaultAge)).toBeLessThan(30);

    const hintBefore = (await RoomScreen.roomExpiryHint(page).textContent()) ?? "";
    expect(hintBefore).not.toBe("");
    const longestAge = allowedAges[allowedAges.length - 1];
    await actor.attemptsTo(ConfigureRoom({ durationSeconds: longestAge }));

    const after = await remainingSeconds(request, room.name);
    expect(Math.abs(after - longestAge)).toBeLessThan(30);
    await expect(RoomScreen.roomExpiryHint(page)).not.toHaveText(hintBefore);
  });

  // 容量上限与新成员默认角色此前只在平台管理面板可改，房间侧边栏缺失。
  // 两个入口共用同一后端核心，侧边栏必须同样能改。
  test("applies capacity and default role changes made in the sidebar", async ({
    actor,
    request,
    provisionRoom,
  }) => {
    const room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-sidebar-policy"),
    });
    await actor.attemptsTo(OpenRoom(room.url));

    const before = await roomView(request, room.name);
    const nextMaxSize = before.max_size + 5 * 1024 * 1024;
    expect(before.default_role_key).not.toBe("editor");

    await actor.attemptsTo(
      ConfigureRoom({ maxSizeBytes: nextMaxSize, defaultRoleKey: "editor" }),
    );

    const after = await roomView(request, room.name);
    expect(after.max_size).toBe(nextMaxSize);
    expect(after.default_role_key).toBe("editor");
  });
});
