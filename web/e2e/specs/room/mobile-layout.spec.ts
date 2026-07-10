import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import {
  OpenRoom,
  SaveMessages,
  SendMessage,
  SwitchToMobileViewport,
} from "../../screenplay/room/tasks/Room.tasks";
import { tRoom } from "../../screenplay/support/i18n";

test.describe("Room mobile layout", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-mobile-layout"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
    await actor.attemptsTo(SwitchToMobileViewport());
  });

  test("keeps mobile chrome compact until a message is selected", async ({
    actor,
    page,
  }) => {
    await expect(RoomScreen.brandLabel(page)).toBeHidden();
    await expect(RoomScreen.messageSelectionToolbar(page)).toBeHidden();
    await expect(RoomScreen.topbarSelectionActions(page)).toBeHidden();

    const tabsBox = await RoomScreen.mobileBottomTabs(page).boundingBox();
    expect(tabsBox?.height ?? 0).toBeLessThanOrEqual(48);

    await actor.attemptsTo(
      SendMessage("Message that can be selected on mobile"),
      SaveMessages(),
    );

    await RoomScreen.messageCheckboxes(page).first().click();
    await expect(RoomScreen.messageSelectionToolbar(page)).toBeVisible();
    await expect(RoomScreen.messageSelectionToolbar(page)).toContainText(
      tRoom("messageList.selectedCount", { count: 1 }),
    );
    await expect(RoomScreen.topbarSelectionActions(page)).toBeVisible();
  });

  test("exposes the project GitHub link without navigating the room", async ({
    page,
  }) => {
    const githubLink = RoomScreen.githubProjectLink(page);

    await expect(githubLink).toHaveAttribute(
      "href",
      "https://github.com/YuniqueUnic/elizabeth",
    );
    await expect(githubLink).toHaveAttribute("target", "_blank");
    await expect(githubLink).toHaveAttribute(
      "aria-label",
      /GitHub|在 GitHub 上打开项目/,
    );
    await expect(githubLink.locator("svg")).toBeVisible();
  });

  test("keeps the chat mounted without reloading messages across mobile tabs", async ({
    actor,
    page,
  }) => {
    await actor.attemptsTo(
      SendMessage("Persistent mobile message"),
      SaveMessages(),
    );

    const message = RoomScreen.messageItems(page).last();
    await message.evaluate((element) => {
      element.setAttribute("data-mount-probe", "preserved");
    });

    let messagePageRequests = 0;
    page.on("request", (request) => {
      const path = new URL(request.url()).pathname;
      if (
        request.method() === "GET" &&
        path.endsWith(`/rooms/${room.name}/messages`)
      ) {
        messagePageRequests += 1;
      }
    });

    await RoomScreen.mobileSettingsTab(page).click();
    await RoomScreen.mobileFilesTab(page).click();
    await RoomScreen.mobileChatTab(page).click();

    await expect(message).toHaveAttribute("data-mount-probe", "preserved");
    expect(messagePageRequests).toBe(0);
  });
});
