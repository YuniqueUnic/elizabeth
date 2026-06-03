import { Question, the } from "@serenity-js/core";

import { CallElizabethApi } from "../../abilities/CallElizabethApi.ability";
import { nativePageFor } from "../../support/actor-page";
import { readClipboard } from "../../support/clipboard";
import { RoomScreen } from "../screens/Room.screen";

export const CurrentUrl = () =>
  Question.about(the`the current page URL`, async (actor) => {
    const page = await nativePageFor(actor);
    return page.url();
  });

export const CurrentRoomName = () =>
  Question.about(the`the current room name`, async (actor) => {
    const page = await nativePageFor(actor);
    const url = new URL(page.url());
    return url.pathname.split("/").filter(Boolean).at(-1) ?? "";
  });

export const MessageCount = () =>
  Question.about(the`the current message count`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageItems(page).count();
  });

export const LastMessageText = () =>
  Question.about(the`the latest message text`, async (actor) => {
    const page = await nativePageFor(actor);
    const text = await RoomScreen.messageContents(page).last().textContent();
    return text?.trim() ?? "";
  });

export const MessageTexts = () =>
  Question.about(the`the message texts`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageContents(page).allInnerTexts();
  });

export const UnsavedBadgeCount = () =>
  Question.about(the`the unsaved badge count`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageUnsavedBadges(page).count();
  });

export const EditedBadgeCount = () =>
  Question.about(the`the edited badge count`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageEditedBadges(page).count();
  });

export const DraftMessageText = () =>
  Question.about(the`the current message draft`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageInput(page).textContent();
  });

export const FileNames = () =>
  Question.about(the`the uploaded file names`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.fileNames(page).allInnerTexts();
  });

export const FileCount = () =>
  Question.about(the`the uploaded file count`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.fileCards(page).count();
  });

export const PreviewedFileName = () =>
  Question.about(the`the previewed file name`, async (actor) => {
    const page = await nativePageFor(actor);
    const text = await RoomScreen.filePreviewTitle(page).textContent();
    return text?.trim() ?? "";
  });

export const ClipboardContents = () =>
  Question.about(the`the clipboard contents`, async (actor) => {
    const page = await nativePageFor(actor);
    return readClipboard(page);
  });

export const RoomCapacitySummary = () =>
  Question.about(the`the room capacity summary`, async (actor) => {
    const page = await nativePageFor(actor);
    const text = await RoomScreen.capacityInfo(page).textContent();
    return text?.trim() ?? "";
  });

export const PermissionState = (label: "预览" | "编辑" | "分享" | "删除") =>
  Question.about(the`whether the ${label} permission is enabled`, async (actor) => {
    const page = await nativePageFor(actor);
    return (await RoomScreen.permissionButton(page, label).getAttribute("aria-pressed")) === "true";
  });

export const AlertText = () =>
  Question.about(the`the current alert text`, async (actor) => {
    const page = await nativePageFor(actor);
    const text = await RoomScreen.alert(page).textContent();
    return text?.trim() ?? "";
  });

export const ScrollMetrics = () =>
  Question.about(the`the message list scroll metrics`, async (actor) => {
    const page = await nativePageFor(actor);
    return RoomScreen.messageListScroll(page).evaluate((element) => {
      const viewport = element.querySelector("[data-radix-scroll-area-viewport]") as HTMLDivElement | null;
      if (!viewport) {
        return {
          clientHeight: 0,
          scrollHeight: 0,
          scrollTop: 0,
        };
      }

      return {
        clientHeight: viewport.clientHeight,
        scrollHeight: viewport.scrollHeight,
        scrollTop: viewport.scrollTop,
      };
    });
  });

export const RoomExists = (roomName: string) =>
  Question.about(the`whether the room ${roomName} still exists`, async (actor) => {
    const api = CallElizabethApi.as(actor);
    return api.roomExists(roomName);
  });
