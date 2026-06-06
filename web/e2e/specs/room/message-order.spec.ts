import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";
import type { ProvisionedRoom } from "../../screenplay/support/constants";
import { uniqueRoomName } from "../../screenplay/support/test-data";
import {
  MessageCount,
  MessageTexts,
  UnsavedBadgeCount,
} from "../../screenplay/room/questions/Room.questions";
import { RoomScreen } from "../../screenplay/room/screens/Room.screen";
import { tRoom } from "../../screenplay/support/i18n";
import {
  OpenRoom,
  SaveMessages,
  SendMessage,
} from "../../screenplay/room/tasks/Room.tasks";
import { Interaction, the } from "@serenity-js/core";
import { nativePageFor } from "../../screenplay/support/actor-page";

const EditMessageAtIndex = (index: number, newContent: string) =>
  Interaction.where(the`#actor edits the message at index ${index} to ${newContent}`, async (actor) => {
    const page = await nativePageFor(actor);
    const item = RoomScreen.messageItems(page).nth(index);
    await item.hover();
    await item.getByRole("button", { name: tRoom("messageBubble.edit") }).click();
    await RoomScreen.messageInput(page).fill(newContent);
    await RoomScreen.sendButton(page).click();
  });

const DeleteMessageAtIndex = (index: number) =>
  Interaction.where(the`#actor deletes the message at index ${index}`, async (actor) => {
    const page = await nativePageFor(actor);
    const item = RoomScreen.messageItems(page).nth(index);
    await item.hover();
    await item.getByRole("button", { name: /delete|删除/i }).click();
    await RoomScreen.deleteConfirmButton(page).click();
  });

const ReloadThePage = () =>
  Interaction.where(the`#actor reloads the page`, async (actor) => {
    const page = await nativePageFor(actor);
    await page.reload();
    // Wait for editor to be ready to confirm page load
    await page.waitForSelector(".tiptap-editor-content [contenteditable='true']");
  });

test.describe("Room message ordering", () => {
  let room: ProvisionedRoom;

  test.beforeEach(async ({ actor, provisionRoom }) => {
    room = await provisionRoom({
      actor,
      roomName: uniqueRoomName("message-order"),
    });

    await actor.attemptsTo(OpenRoom(room.url));
  });

  test("preserves exact sequence for multiple messages sent in quick succession", async ({ actor }) => {
    const initialCount = await actor.answer(MessageCount());

    // Send multiple messages quickly (they remain unsaved in UI)
    await actor.attemptsTo(
      SendMessage("Msg 1"),
      SendMessage("Msg 2"),
      SendMessage("Msg 3"),
      SendMessage("Msg 4"),
      SendMessage("Msg 5"),
    );

    // Verify they are added locally in order
    await expect.poll(async () => actor.answer(MessageCount())).toBe(initialCount + 5);
    const textsBeforeSave = await actor.answer(MessageTexts());
    expect(textsBeforeSave.slice(-5)).toEqual([
      "Msg 1",
      "Msg 2",
      "Msg 3",
      "Msg 4",
      "Msg 5",
    ]);

    // Save and wait for unsaved badge count to be 0 (completed sequential saving)
    await actor.attemptsTo(SaveMessages());
    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);

    // Reload page
    await actor.attemptsTo(ReloadThePage());

    // Verify messages persist in the exact same chronological order
    const textsAfterReload = await actor.answer(MessageTexts());
    expect(textsAfterReload.slice(-5)).toEqual([
      "Msg 1",
      "Msg 2",
      "Msg 3",
      "Msg 4",
      "Msg 5",
    ]);
  });

  test("preserves correct order when mixing edits and new messages", async ({ actor }) => {
    // Send initial messages and save them
    await actor.attemptsTo(
      SendMessage("First Message"),
      SendMessage("Second Message"),
      SendMessage("Third Message"),
      SaveMessages(),
    );
    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);

    // Edit the second message, then add two new unsaved messages
    await actor.attemptsTo(
      EditMessageAtIndex(1, "Second Message (Edited)"),
      SendMessage("Fourth Message"),
      SendMessage("Fifth Message"),
    );

    // Verify local state before saving
    const textsLocal = await actor.answer(MessageTexts());
    expect(textsLocal.slice(-5)).toEqual([
      "First Message",
      "Second Message (Edited)",
      "Third Message",
      "Fourth Message",
      "Fifth Message",
    ]);

    // Save and wait for saving to finish
    await actor.attemptsTo(SaveMessages());
    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);

    // Reload page
    await actor.attemptsTo(ReloadThePage());

    // Verify server-side state matches the correct sequence
    const textsServer = await actor.answer(MessageTexts());
    expect(textsServer.slice(-5)).toEqual([
      "First Message",
      "Second Message (Edited)",
      "Third Message",
      "Fourth Message",
      "Fifth Message",
    ]);
  });

  test("preserves correct order when mixing deletions, edits, and new messages", async ({ actor }) => {
    // Send initial messages and save
    await actor.attemptsTo(
      SendMessage("Item A"),
      SendMessage("Item B"),
      SendMessage("Item C"),
      SaveMessages(),
    );
    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);

    // Delete the second message (index 1).
    // Note: Pending delete message remains in DOM (with opacity-50 and line-through) until saved.
    // So "Item C" remains at index 2 in the DOM list of items. We edit Item C at index 2.
    await actor.attemptsTo(
      DeleteMessageAtIndex(1),
      EditMessageAtIndex(2, "Item C (Edited)"),
      SendMessage("Item D"),
    );

    // Verify local state before saving (includes Item B since it's only marked for deletion in DOM)
    const textsLocal = await actor.answer(MessageTexts());
    expect(textsLocal.slice(-4)).toEqual([
      "Item A",
      "Item B",
      "Item C (Edited)",
      "Item D",
    ]);

    // Save and wait for saving to finish
    await actor.attemptsTo(SaveMessages());
    await expect.poll(async () => actor.answer(UnsavedBadgeCount())).toBe(0);

    // Reload page
    await actor.attemptsTo(ReloadThePage());

    // Verify server-side state matches the correct sequence (Item B is now physically removed)
    const textsServer = await actor.answer(MessageTexts());
    expect(textsServer.slice(-3)).toEqual([
      "Item A",
      "Item C (Edited)",
      "Item D",
    ]);
  });
});
