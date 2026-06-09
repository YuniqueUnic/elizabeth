import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { readNotifications, setNotificationPermission } from "../../screenplay/support/notifications";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { MessageCount } from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  ConfirmDelete,
  DeleteMessageById,
  OpenRoom,
  SaveMessages,
  SendMessage,
  SetSettingTo,
} from "../../screenplay/room/tasks/Room.tasks";
import { tCommon } from "../../screenplay/support/i18n";

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
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => `${notification.title} ${notification.options?.body ?? ""}`)
        .join("\n");
    }).toContain("Remote notification payload");
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
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => `${notification.title} ${notification.options?.body ?? ""}`)
        .join("\n");
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
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => `${notification.title} ${notification.options?.body ?? ""}`)
        .join("\n");
    }).toContain(tCommon("desktopNotification.title.message.deleted"));

    await expect.poll(async () => {
      const notifications = await readNotifications(page);
      return notifications
        .map((notification) => notification.options?.body ?? "")
        .join("\n");
    }).toContain(message);
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
