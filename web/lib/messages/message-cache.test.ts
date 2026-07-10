import assert from "node:assert/strict";
import { describe, test } from "node:test";

import type { LocalMessage, Message } from "../types";
import {
  compareMessageOrder,
  mergeMessagePage,
  messageFromContentEvent,
  rebasePendingMessages,
  removeMessage,
  replacePendingMessage,
  replaceSavedMessage,
} from "./message-cache";

const serverMessage = (
  id: string,
  sequence: number,
  content = `message-${id}`,
  updatedAt = "2026-07-10T00:00:00Z",
): Message => ({
  id,
  content,
  timestamp: "2026-07-10T00:00:00Z",
  updatedAt,
  sequence_number: sequence,
});

describe("message cache", () => {
  test("sorts by sequence number and numeric id", () => {
    const messages = [
      serverMessage("10", 2),
      serverMessage("9", 2),
      serverMessage("100", 1),
    ];

    assert.deepEqual(messages.sort(compareMessageOrder).map((message) => message.id), [
      "100",
      "9",
      "10",
    ]);
  });

  test("merges overlapping pages without duplicates", () => {
    const existing = [serverMessage("3", 3), serverMessage("4", 4)];
    const merged = mergeMessagePage(existing, [
      serverMessage("1", 1),
      serverMessage("2", 2),
      serverMessage("3", 3),
    ]);

    assert.deepEqual(merged.map((message) => message.id), ["1", "2", "3", "4"]);
    assert.equal(merged[2], existing[0]);
    assert.equal(merged[3], existing[1]);
  });

  test("does not overwrite local edits with a page response", () => {
    const dirty: LocalMessage = {
      ...serverMessage("7", 7, "local edit"),
      isDirty: true,
      originalContent: "server value",
    };

    const merged = mergeMessagePage([dirty], [serverMessage("7", 7, "server value")]);
    assert.deepEqual(merged, [dirty]);
  });

  test("ignores an older server revision and replaces a newer revision", () => {
    const current = serverMessage("8", 8, "current", "2026-07-10T10:00:00Z");
    const older = serverMessage("8", 8, "older", "2026-07-10T09:00:00Z");
    const newer = serverMessage("8", 8, "newer", "2026-07-10T11:00:00Z");

    assert.equal(mergeMessagePage([current], [older])[0], current);
    assert.equal(mergeMessagePage([current], [newer])[0]?.content, "newer");
  });

  test("replaces a temporary message and removes websocket duplicates", () => {
    const pending: LocalMessage = {
      ...serverMessage("temp-1", 10, "pending"),
      isNew: true,
    };
    const saved = serverMessage("42", 10, "pending");

    const messages = replacePendingMessage([pending, saved], pending.id, saved);
    assert.equal(messages.length, 1);
    assert.equal(messages[0]?.id, "42");
    assert.equal(messages[0]?.isNew, false);
  });

  test("accepts an authoritative save response and clears local edit state", () => {
    const dirty: LocalMessage = {
      ...serverMessage("7", 7, "local edit"),
      isDirty: true,
      originalContent: "server value",
    };
    const saved = serverMessage("7", 7, "local edit", "2026-07-10T01:00:00Z");

    const messages = replaceSavedMessage([dirty], saved);

    assert.equal(messages.length, 1);
    assert.equal(messages[0]?.content, "local edit");
    assert.equal(messages[0]?.isDirty, false);
    assert.equal(messages[0]?.originalContent, undefined);
  });

  test("keeps local temporary messages after ordered server messages", () => {
    const pending: LocalMessage = {
      ...serverMessage("temp-1", 0, "pending"),
      isNew: true,
    };

    const messages = mergeMessagePage([pending], [serverMessage("2", 2)]);
    assert.deepEqual(messages.map((message) => message.id), ["2", "temp-1"]);
  });

  test("builds an updated message event using cached fields", () => {
    const existing = serverMessage("12", 4, "before");
    const message = messageFromContentEvent({
      content_id: 12,
      room_name: "room",
      text: "after",
      updated_at: "2026-07-10T12:00:00Z",
    }, existing);

    assert.deepEqual(message && {
      id: message.id,
      content: message.content,
      timestamp: message.timestamp,
      sequence_number: message.sequence_number,
    }, {
      id: "12",
      content: "after",
      timestamp: existing.timestamp,
      sequence_number: 4,
    });
  });

  test("removes only the target message", () => {
    const first = serverMessage("1", 1);
    const second = serverMessage("2", 2);
    assert.deepEqual(removeMessage([first, second], "1"), [second]);
  });

  test("rebases pending messages after the latest server sequence", () => {
    const pendingOne: LocalMessage = {
      ...serverMessage("temp-1", 0),
      isNew: true,
    };
    const pendingTwo: LocalMessage = {
      ...serverMessage("temp-2", 1),
      isNew: true,
    };

    const result = rebasePendingMessages([
      serverMessage("20", 20),
      pendingOne,
      pendingTwo,
    ], 21);

    assert.deepEqual(
      result.messages.map((message) => message.sequence_number),
      [20, 21, 22],
    );
    assert.equal(result.nextSequenceNumber, 23);
  });
});
