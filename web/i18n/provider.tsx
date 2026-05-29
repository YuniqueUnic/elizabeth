"use client";

import { NextIntlClientProvider } from "next-intl";
import { useState, useEffect } from "react";
import { useAppStore } from "@/lib/store";
import { loadMessages, type MessageNamespace } from "@/messages";

type Messages = Record<MessageNamespace, Record<string, unknown>>;

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const locale = useAppStore((s) => s.locale);
  const [messages, setMessages] = useState<Messages | null>(null);

  // Sync <html lang> and load messages when locale changes
  useEffect(() => {
    document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
    let cancelled = false;
    loadMessages(locale).then((msgs) => {
      if (!cancelled) setMessages(msgs as Messages);
    });
    return () => { cancelled = true; };
  }, [locale]);

  if (!messages) return null;

  return (
    <NextIntlClientProvider locale={locale} messages={messages}>
      {children}
    </NextIntlClientProvider>
  );
}
