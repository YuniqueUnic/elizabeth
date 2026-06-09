import type { BrowserContext, Page } from "@playwright/test";
import { actorCalled, type Actor, TakeNotes } from "@serenity-js/core";
import { BrowseTheWebWithPlaywright } from "@serenity-js/playwright";
import {
  useFixtures,
} from "@serenity-js/playwright-test";

import { CallElizabethApi } from "../abilities/CallElizabethApi.ability";
import { nativePageFor } from "../support/actor-page";
import { installClipboardStub } from "../support/clipboard";
import { installNotificationStub } from "../support/notifications";
import {
  APP_BASE_URL,
  type ProvisionedRoom,
  type RoomTokenInfo,
} from "../support/constants";
import { uniqueRoomName } from "../support/test-data";
import { primeRoomToken } from "../support/token-storage";

export interface ActorHandle {
  actor: Actor;
  page: Page;
  context: BrowserContext;
}

interface ProvisionRoomOptions {
  actor?: Actor;
  roomName?: string;
  password?: string;
  injectToken?: boolean;
  withRefreshToken?: boolean;
}

interface ScreenplayFixtures {
  createActor: (name: string) => Promise<ActorHandle>;
  provisionRoom: (options?: ProvisionRoomOptions) => Promise<ProvisionedRoom>;
}

const {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  test,
} = useFixtures<ScreenplayFixtures>({
  page: async ({ page }, use) => {
    await installClipboardStub(page);
    await installNotificationStub(page);
    await use(page);
  },

  extraAbilities: async ({ request }, use) => {
    await use([CallElizabethApi.using(request)]);
  },

  createActor: async ({ browser, request }, use) => {
    const contexts: BrowserContext[] = [];

    const createActor = async (name: string): Promise<ActorHandle> => {
      const context = await browser.newContext();
      contexts.push(context);
      await installClipboardStub(context);
      await installNotificationStub(context);

      const page = await context.newPage();
      const actor = actorCalled(name).whoCan(
        BrowseTheWebWithPlaywright.usingPage(page),
        TakeNotes.usingAnEmptyNotepad(),
        CallElizabethApi.using(request),
      );

      return {
        actor,
        page,
        context,
      };
    };

    await use(createActor);

    await Promise.all(
      contexts.map(async (context) => {
        await context.close().catch(() => {});
      }),
    );
  },

  provisionRoom: async ({ request }, use) => {
    const api = CallElizabethApi.using(request);

    const provisionRoom = async (
      options: ProvisionRoomOptions = {},
    ): Promise<ProvisionedRoom> => {
      const roomName = options.roomName ?? uniqueRoomName("screenplay-room");
      await api.ensureRoom(roomName, options.password);

      let tokenInfo: RoomTokenInfo | undefined;
      if (options.injectToken !== false && options.actor) {
        tokenInfo = await api.issueToken(roomName, {
          password: options.password,
          withRefreshToken: options.withRefreshToken,
        });
        const page = await nativePageFor(options.actor);
        await primeRoomToken(page, roomName, tokenInfo);
      }

      return {
        name: roomName,
        password: options.password,
        tokenInfo,
        url: `${APP_BASE_URL}/${roomName}`,
      };
    };

    await use(provisionRoom);
  },
});

export { afterAll, beforeAll, beforeEach, describe, expect, it, test };
