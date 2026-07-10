import type { LocalMessage, Message } from "../types";

export interface MessageContentEvent {
  content_id: number | null;
  room_name: string;
  text?: string | null;
  sequence_number?: number;
  created_at?: string;
  updated_at?: string;
}

const isPending = (message: LocalMessage) =>
  Boolean(message.isNew || message.isDirty || message.isPendingDelete);

const numericId = (id: string) => {
  const value = Number(id);
  return Number.isFinite(value) ? value : Number.MAX_SAFE_INTEGER;
};

export function compareMessageOrder(left: Message, right: Message): number {
  const sequenceDifference = (left.sequence_number ?? 0) -
    (right.sequence_number ?? 0);
  if (sequenceDifference !== 0) return sequenceDifference;

  const idDifference = numericId(left.id) - numericId(right.id);
  if (idDifference !== 0) return idDifference;
  return left.id.localeCompare(right.id);
}

const sameServerMessage = (left: LocalMessage, right: LocalMessage) =>
  left.id === right.id &&
  left.content === right.content &&
  left.timestamp === right.timestamp &&
  left.updatedAt === right.updatedAt &&
  left.fileName === right.fileName &&
  left.user === right.user &&
  left.isOwn === right.isOwn &&
  left.sequence_number === right.sequence_number &&
  !isPending(left);

function normalizeServerMessage(
  incoming: Message,
  existing?: LocalMessage,
): LocalMessage {
  const normalized: LocalMessage = {
    ...incoming,
    isOwn: incoming.isOwn ?? existing?.isOwn,
    isNew: false,
    isDirty: false,
    isPendingDelete: false,
    originalContent: undefined,
  };

  return existing && sameServerMessage(existing, normalized)
    ? existing
    : normalized;
}

function chooseServerVersion(
  existing: LocalMessage | undefined,
  incoming: Message,
): LocalMessage {
  if (!existing) return normalizeServerMessage(incoming);
  if (isPending(existing)) return existing;

  const existingRevision = existing.updatedAt ?? existing.timestamp;
  const incomingRevision = incoming.updatedAt ?? incoming.timestamp;
  if (incomingRevision < existingRevision) return existing;
  return normalizeServerMessage(incoming, existing);
}

export function mergeMessagePage(
  existing: LocalMessage[],
  incoming: Message[],
): LocalMessage[] {
  const byId = new Map(existing.map((message) => [message.id, message]));
  for (const message of incoming) {
    byId.set(message.id, chooseServerVersion(byId.get(message.id), message));
  }

  const serverMessages = [...byId.values()]
    .filter((message) => !message.isNew)
    .sort(compareMessageOrder);
  const localMessages = existing.filter((message) => message.isNew);

  return [...serverMessages, ...localMessages];
}

export function replacePendingMessage(
  messages: LocalMessage[],
  pendingId: string,
  saved: Message,
): LocalMessage[] {
  const withoutPendingOrDuplicate = messages.filter((message) =>
    message.id !== pendingId && message.id !== saved.id
  );
  return mergeMessagePage(withoutPendingOrDuplicate, [saved]);
}

export function rebasePendingMessages(
  messages: LocalMessage[],
  serverNextSequenceNumber: number,
): { messages: LocalMessage[]; nextSequenceNumber: number } {
  const highestServerSequence = messages.reduce(
    (highest, message) => message.isNew
      ? highest
      : Math.max(highest, message.sequence_number ?? 0),
    -1,
  );
  const firstPendingSequence = Math.max(
    serverNextSequenceNumber,
    highestServerSequence + 1,
  );
  let pendingOffset = 0;
  const rebased = messages.map((message) => {
    if (!message.isNew) return message;
    const sequenceNumber = firstPendingSequence + pendingOffset;
    pendingOffset += 1;
    return message.sequence_number === sequenceNumber
      ? message
      : { ...message, sequence_number: sequenceNumber };
  });

  return {
    messages: rebased,
    nextSequenceNumber: firstPendingSequence + pendingOffset,
  };
}

export function removeMessage(messages: LocalMessage[], id: string): LocalMessage[] {
  return messages.filter((message) => message.id !== id);
}

export function messageFromContentEvent(
  payload: MessageContentEvent,
  existing?: LocalMessage,
): Message | null {
  if (payload.content_id == null) return null;
  const content = payload.text ?? existing?.content;
  const timestamp = payload.created_at ?? existing?.timestamp;
  if (content == null || timestamp == null) return null;

  return {
    id: String(payload.content_id),
    content,
    timestamp,
    updatedAt: payload.updated_at ?? payload.created_at ?? existing?.updatedAt,
    sequence_number: payload.sequence_number ?? existing?.sequence_number ?? 0,
    isOwn: existing?.isOwn,
  };
}
