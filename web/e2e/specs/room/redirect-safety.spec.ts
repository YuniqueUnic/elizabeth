import type { Page, Route } from "@playwright/test";

import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { tCommon } from "../../screenplay/support/i18n";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import { PermissionState } from "../../screenplay/room/questions/Room.questions";
import {
  ConfigureRoom,
  OpenRoom,
  SendMessage,
  SetRoomPermissions,
} from "../../screenplay/room/tasks/Room.tasks";

async function rewriteSlugOnMutation(
  page: Page,
  newSlug: string,
): Promise<void> {
  await page.route(/\/(permissions|settings)/, async (route: Route) => {
    if (!["POST", "PUT"].includes(route.request().method())) {
      return route.continue();
    }

    const response = await route.fetch();
    const json = await response.json();

    await route.fulfill({
      json: {
        ...json,
        name: newSlug,
        slug: newSlug,
      },
      response,
    });
  });
}

test.describe("Room redirect safety", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("screenplay-redirect"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("warns about unsaved messages when the settings save returns a new room slug", async ({
    actor,
    page,
  }) => {
    const newSlug = `${room.name}-renamed`;

    await actor.attemptsTo(SendMessage("This is an unsaved message"));
    await rewriteSlugOnMutation(page, newSlug);
    await actor.attemptsTo(ConfigureRoom({ maxViews: 999 }));
    await page.keyboard.press("Escape").catch(() => {});

    await expect(RoomScreen.roomAddressChangedAlert(page)).toBeVisible();
    await expect(RoomScreen.roomAddressChangedAlert(page)).toContainText(tCommon("unsavedChangesWarning"));
    await expect(RoomScreen.roomAddressChangedAlert(page)).toContainText(`/${newSlug}`);
  });

  test("warns about unsaved messages when a permission change returns a new room slug", async ({
    actor,
    page,
  }) => {
    const newSlug = `${room.name}-perm-renamed`;
    const shareEnabled = await actor.answer(PermissionState("share"));

    await actor.attemptsTo(SendMessage("Message before permission change"));
    await rewriteSlugOnMutation(page, newSlug);
    await actor.attemptsTo(SetRoomPermissions({ share: !shareEnabled }));

    await expect(RoomScreen.roomAddressChangedAlert(page)).toBeVisible();
    await expect(RoomScreen.roomAddressChangedAlert(page)).toContainText(tCommon("unsavedChangesWarning"));
    await expect(RoomScreen.roomAddressChangedAlert(page)).toContainText(`/${newSlug}`);
  });
});
