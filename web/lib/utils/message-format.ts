"use client";

import type { Message } from "@/lib/types";
import { formatDate } from "@/lib/utils/format";

export interface MessageFormatOptions {
  includeMetadata: boolean;
  tHeader: (params: { number: number }) => string;
  tUser: (params: { user: string }) => string;
  tAnonymous: string;
  tTime: (params: { time: string }) => string;
}

export function formatMessagesMarkdown(
  messages: Message[],
  selectedIds: Set<string>,
  options: MessageFormatOptions,
): string {
  const { includeMetadata, tHeader, tUser, tAnonymous, tTime } = options;

  return messages
    .filter((m) => selectedIds.has(m.id))
    .map((m) => {
      if (includeMetadata) {
        const messageNumber = messages.indexOf(m) + 1;
        return `### ${tHeader({ number: messageNumber })}\n**${tUser({ user: m.user || tAnonymous })}**\n**${tTime({ time: formatDate(m.timestamp) })}**\n\n${m.content}`;
      }
      return m.content;
    })
    .join("\n\n---\n\n");
}

export function formatSingleMessageMarkdown(
  message: Message,
  messageNumber: number,
  options: MessageFormatOptions,
): string {
  const { includeMetadata, tHeader, tUser, tAnonymous, tTime } = options;

  if (includeMetadata) {
    return `### ${tHeader({ number: messageNumber })}\n**${tUser({ user: message.user || tAnonymous })}**\n**${tTime({ time: formatDate(message.timestamp) })}**\n\n${message.content}`;
  }
  return message.content;
}

export function downloadMarkdown(content: string, filenamePrefix = "messages"): void {
  const blob = new Blob([content], { type: "text/markdown" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${filenamePrefix}-${new Date().toISOString().split("T")[0]}.md`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
