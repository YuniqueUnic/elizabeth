import type { Locale } from "@/i18n/config";

const messageModules = {
  zh: {
    common: () => import("./zh/common.json").then((m) => m.default),
    home: () => import("./zh/home.json").then((m) => m.default),
    room: () => import("./zh/room.json").then((m) => m.default),
    errors: () => import("./zh/errors.json").then((m) => m.default),
    settings: () => import("./zh/settings.json").then((m) => m.default),
    help: () => import("./zh/help.json").then((m) => m.default),
  },
  en: {
    common: () => import("./en/common.json").then((m) => m.default),
    home: () => import("./en/home.json").then((m) => m.default),
    room: () => import("./en/room.json").then((m) => m.default),
    errors: () => import("./en/errors.json").then((m) => m.default),
    settings: () => import("./en/settings.json").then((m) => m.default),
    help: () => import("./en/help.json").then((m) => m.default),
  },
} as const;

export type MessageNamespace = keyof (typeof messageModules)["zh"];

export async function loadMessages(locale: Locale) {
  const mods = messageModules[locale];
  const [common, home, room, errors, settings, help] = await Promise.all([
    mods.common(),
    mods.home(),
    mods.room(),
    mods.errors(),
    mods.settings(),
    mods.help(),
  ]);
  return { common, home, room, errors, settings, help };
}
