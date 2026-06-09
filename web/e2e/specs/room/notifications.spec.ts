import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { readNotifications, setNotificationPermission } from "../../screenplay/support/notifications";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  FileNames,
  LastMessageText,
  MessageCount,
  PermissionState,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  AddRoomLink,
  ConfigureRoom,
  ConfirmDelete,
  DeleteMessageById,
  OpenRoom,
  SaveMessages,
  SendMessage,
  SetSettingTo,
  SetRoomPermissions,
  UpdateLatestMessage,
} from "../../screenplay/room/tasks/Room.tasks";
import { tCommon } from "../../screenplay/support/i18n";

const notificationText = async (page: import("@playwright/test").Page) => {
  const notifications = await readNotifications(page);
  return notifications
    .map((notification) => `${notification.title} ${notification.options?.body ?? ""}`)
    .join("\n");
};

const notificationTags = async (page: import("@playwright/test").Page) => {
  const notifications = await readNotifications(page);
  return notifications
    .map((notification) => notification.options?.tag ?? "")
    .join("\n");
};

test.describe("Browser desktop notifications", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-notifications"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("sends a desktop notification for remote message changes when enabled", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const sender = await createActor("notification sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage("Remote notification payload"),
      SaveMessages(),
    );

    await expect.poll(async () => {
      return notificationText(page);
    }).toContain("Remote notification payload");
  });

  test("can hide the concrete message body in desktop notifications", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(
      SetSettingTo("setting-desktop-notifications", true),
      SetSettingTo("setting-desktop-notification-show-content", false),
    );

    const sensitiveMessage = `DO_NOT_LEAK_MESSAGE_${Date.now()}`;
    const sender = await createActor("notification privacy sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage(sensitiveMessage),
      SaveMessages(),
    );

    await expect.poll(async () => actor.answer(MessageCount())).toBe(1);
    await expect.poll(async () => readNotifications(page).then((items) => items.length))
      .toBeGreaterThan(0);

    const text = await notificationText(page);
    expect(text).not.toContain(sensitiveMessage);
    expect(text).toContain(room.name);
    expect(text).toContain(tCommon("desktopNotification.summary.message.created"));
  });

  test("does not notify when the desktop notification setting is off", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");

    const sender = await createActor("notification disabled sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage("Disabled notification payload"),
      SaveMessages(),
    );

    await expect.poll(async () => actor.answer(MessageCount())).toBe(1);
    expect(await readNotifications(page)).toHaveLength(0);
  });

  test("sends a desktop notification with message details when a remote message is deleted", async ({
    actor,
    page,
    createActor,
  }) => {
    const message = "Remote deleted notification payload";
    const sender = await createActor("notification delete sender");

    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage(message),
      SaveMessages(),
    );
    await expect.poll(async () => actor.answer(MessageCount())).toBe(1);

    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const messageId = ((await RoomScreen.messageItems(sender.page)
      .last()
      .getAttribute("data-testid")) ?? "").replace("message-item-", "");
    expect(messageId).not.toBe("");

    await sender.actor.attemptsTo(DeleteMessageById(messageId));
    await expect(RoomScreen.deleteConfirmDialog(sender.page)).toBeVisible();
    await sender.actor.attemptsTo(
      ConfirmDelete(),
      SaveMessages(),
    );

    await expect.poll(async () => {
      return notificationText(page);
    }).toContain(tCommon("desktopNotification.title.message.deleted"));

    await expect.poll(async () => {
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => notification.options?.body ?? "")
        .join("\n");
    }).toContain(message);
  });

  test("respects individual message notification type switches", async ({
    actor,
    page,
    createActor,
  }) => {
    const message = "Matrix notification payload";

    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(
      SetSettingTo("setting-desktop-notifications", true),
      SetSettingTo("setting-desktop-notification-message-created", false),
    );

    const sender = await createActor("notification matrix sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage(message),
      SaveMessages(),
    );
    await expect.poll(async () => actor.answer(MessageCount())).toBe(1);
    expect(await readNotifications(page)).toHaveLength(0);

    const messageId = ((await RoomScreen.messageItems(sender.page)
      .last()
      .getAttribute("data-testid")) ?? "").replace("message-item-", "");
    expect(messageId).not.toBe("");

    await sender.actor.attemptsTo(DeleteMessageById(messageId));
    await expect(RoomScreen.deleteConfirmDialog(sender.page)).toBeVisible();
    await sender.actor.attemptsTo(
      ConfirmDelete(),
      SaveMessages(),
    );

    await expect.poll(async () => {
      return notificationText(page);
    }).toContain(tCommon("desktopNotification.title.message.deleted"));

    await expect.poll(async () => {
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => notification.options?.body ?? "")
        .join("\n");
    }).toContain(message);
  });

  test("sends desktop notifications for remote message updates", async ({
    actor,
    page,
    createActor,
  }) => {
    const sender = await createActor("notification update sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      SendMessage("Original message before update"),
      SaveMessages(),
    );
    await expect.poll(async () => actor.answer(MessageCount())).toBe(1);

    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const updatedMessage = `Remote updated notification payload ${Date.now()}`;
    await sender.actor.attemptsTo(
      UpdateLatestMessage(updatedMessage),
      SaveMessages(),
    );

    await expect.poll(async () => actor.answer(LastMessageText()))
      .toContain(updatedMessage);
    await expect.poll(async () => notificationTags(page))
      .toContain(":message:updated:");
    await expect.poll(async () => notificationText(page))
      .toContain(updatedMessage);
  });

  test("sends desktop notifications for remote room setting updates", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const sender = await createActor("notification room settings sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      ConfigureRoom({ maxViews: 777 }),
    );

    await expect(RoomScreen.maxViewsInput(page)).toHaveValue("777");
    await expect.poll(async () => notificationTags(page))
      .toContain(":room:settings_changed:");
    await expect.poll(async () => notificationText(page))
      .toContain(tCommon("desktopNotification.title.room.settings_changed"));
  });

  test("sends desktop notifications for remote room permission updates", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const sender = await createActor("notification room permissions sender");
    await sender.actor.attemptsTo(OpenRoom(room.url));
    expect(await sender.actor.answer(PermissionState("delete"))).toBe(true);

    await sender.actor.attemptsTo(SetRoomPermissions({ delete: false }));

    await expect(RoomScreen.roomAddressChangedAlert(page)).toHaveCount(0);
    await expect.poll(async () => notificationTags(page))
      .toContain(":room:permissions_changed:");
    await expect.poll(async () => notificationText(page))
      .toContain(tCommon("desktopNotification.title.room.permissions_changed"));
  });

  test("sends desktop notifications for remote room address changes", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    const sender = await createActor("notification room address sender");
    await sender.actor.attemptsTo(OpenRoom(room.url));
    const shareEnabled = await sender.actor.answer(PermissionState("share"));
    expect(shareEnabled).toBe(true);

    await sender.actor.attemptsTo(SetRoomPermissions({ share: false }));

    await expect(RoomScreen.roomAddressChangedAlert(page)).toBeVisible();
    await expect.poll(async () => notificationTags(page))
      .toContain(":room:address_changed:");
    await expect.poll(async () => notificationText(page))
      .toContain(tCommon("desktopNotification.title.room.address_changed"));
    const addressSubjectPrefix = tCommon(
      "desktopNotification.roomUpdateSubject.addressChanged",
      { path: "/__next_room__" },
    ).replace("/__next_room__", "");
    await expect.poll(async () => notificationText(page))
      .toContain(addressSubjectPrefix);
  });

  test("respects the room setting update notification switch", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(
      SetSettingTo("setting-desktop-notifications", true),
      SetSettingTo("setting-desktop-notification-room-settings_changed", false),
    );

    const sender = await createActor("notification room setting disabled sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      ConfigureRoom({ maxViews: 888 }),
    );

    await expect(RoomScreen.maxViewsInput(page)).toHaveValue("888");
    expect(await readNotifications(page)).toHaveLength(0);
  });

  test("does not treat a newly added link as a link update", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(
      SetSettingTo("setting-desktop-notifications", true),
      SetSettingTo("setting-desktop-notification-link-created", false),
    );

    const linkName = `Link created off ${Date.now()}`;
    const sender = await createActor("notification link disabled sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      AddRoomLink({
        urlInput: `example.com/disabled-${Date.now()}`,
        name: linkName,
        description: "Link created switch is disabled",
      }),
    );

    await expect.poll(async () => actor.answer(FileNames())).toContain(linkName);
    expect(await readNotifications(page)).toHaveLength(0);
  });

  test("sends one link-created notification without a file-created duplicate", async ({
    actor,
    page,
    createActor,
  }) => {
    await setNotificationPermission(page, "granted");
    await actor.attemptsTo(
      SetSettingTo("setting-desktop-notifications", true),
      SetSettingTo("setting-desktop-notification-file-created", false),
    );

    const linkName = `Link created only ${Date.now()}`;
    const sender = await createActor("notification link sender");
    await sender.actor.attemptsTo(
      OpenRoom(room.url),
      AddRoomLink({
        urlInput: `example.com/created-${Date.now()}`,
        name: linkName,
        description: "Link created notification should not duplicate file events",
      }),
    );

    await expect.poll(async () => actor.answer(FileNames())).toContain(linkName);
    await expect.poll(async () => notificationTags(page))
      .toContain(":link:created:");

    const tags = await notificationTags(page);
    expect(tags).not.toContain(":file:created:");
    expect(tags).not.toContain(":link:updated:");
    expect((await readNotifications(page))).toHaveLength(1);
  });

  test("keeps the setting off when the browser denies permission", async ({
    actor,
    page,
  }) => {
    await setNotificationPermission(page, "denied");
    await actor.attemptsTo(SetSettingTo("setting-desktop-notifications", true));

    await RoomScreen.settingsButton(page).click();
    await RoomScreen.settingsTab(page, "notifications").click();
    await expect(RoomScreen.settingDesktopNotifications(page)).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });
});
