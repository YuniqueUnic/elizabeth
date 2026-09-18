// Utility functions for formatting

/**
 * 解析后端返回的时间：naive datetime 表示 UTC，缺少时区标识时补 Z；
 * 纳秒精度需截断到毫秒才能被 Date 解析。解析失败返回 null。
 */
export function parseBackendDateTime(value: string): Date | null {
  const normalized = value.replace(/(\.\d{3})\d+$/, "$1");
  const hasTimezone = /(?:[zZ]|[+-]\d{2}:?\d{2})$/.test(normalized);
  const date = new Date(hasTimezone ? normalized : `${normalized}Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

/** 后端时间 → 本地化文案；解析失败时回退原始字符串。 */
export function formatBackendDateTime(
  value: string,
  options: Intl.DateTimeFormatOptions = {
    dateStyle: "medium",
    timeStyle: "short",
  },
): string {
  const date = parseBackendDateTime(value);
  return date ? date.toLocaleString(undefined, options) : value;
}

const DURATION_UNITS = [
  { seconds: 365 * 24 * 60 * 60, unit: "year" },
  { seconds: 24 * 60 * 60, unit: "day" },
  { seconds: 60 * 60, unit: "hour" },
  { seconds: 60, unit: "minute" },
  { seconds: 1, unit: "second" },
] as const;

/** 时长（秒）→ 本地化文案，例如 7200 → “2 小时”。 */
export function formatDuration(totalSeconds: number, locale: string): string {
  const unit =
    DURATION_UNITS.find(
      ({ seconds }) => totalSeconds >= seconds && totalSeconds % seconds === 0,
    ) ?? DURATION_UNITS[DURATION_UNITS.length - 1];
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: unit.unit,
    unitDisplay: "long",
  }).format(totalSeconds / unit.seconds);
}

/** 时长（秒）→ 紧凑令牌，例如 7200 → “2h”；用于与配置文件同形的策略编辑。 */
const DURATION_TOKENS = [
  { seconds: 7 * 24 * 60 * 60, suffix: "w" },
  { seconds: 24 * 60 * 60, suffix: "d" },
  { seconds: 60 * 60, suffix: "h" },
  { seconds: 60, suffix: "m" },
  { seconds: 1, suffix: "s" },
] as const;

const DURATION_TOKEN_UNITS: Record<string, number> = {
  s: 1,
  m: 60,
  h: 60 * 60,
  d: 24 * 60 * 60,
  w: 7 * 24 * 60 * 60,
};

/** 解析时长令牌列表（“1m, 30m, 2h, 7d”，单位 s/m/h/d/w）。
 * 与部署配置 `app.room.expiry.allowed_ages` 的写法一致；任一项非法返回 null。 */
export function parseDurationList(raw: string): number[] | null {
  const entries = raw
    .split(/[,，\s]+/)
    .map((entry) => entry.trim().toLowerCase())
    .filter((entry) => entry.length > 0);
  if (entries.length === 0) return [];
  const seconds: number[] = [];
  for (const entry of entries) {
    const match = /^(\d+)([smhdw])$/.exec(entry);
    if (!match) return null;
    const value = Number(match[1]) * DURATION_TOKEN_UNITS[match[2]];
    if (!Number.isSafeInteger(value) || value <= 0) return null;
    seconds.push(value);
  }
  return seconds;
}

/** 时长（秒）数组 → 令牌列表文案，例如 [60, 7200] → “1m, 2h”。 */
export function formatDurationList(seconds: number[]): string {
  return seconds
    .map((value) => {
      const token = DURATION_TOKENS.find(
        ({ seconds: unit }) => value >= unit && value % unit === 0,
      );
      return token ? `${value / token.seconds}${token.suffix}` : `${value}s`;
    })
    .join(", ");
}

export function formatFileSize(bytes?: number): string {
  if (!bytes) return "未知大小";

  const kb = bytes / 1024;
  const mb = kb / 1024;
  const gb = mb / 1024;

  if (gb >= 1) {
    return `${gb.toFixed(2)} GB`;
  }
  if (mb >= 1) {
    return `${mb.toFixed(2)} MB`;
  }
  if (kb >= 1) {
    return `${kb.toFixed(2)} KB`;
  }
  return `${bytes} B`;
}

export function formatDate(date: string | Date): string {
  const d = typeof date === "string" ? new Date(date) : date;
  const now = new Date();

  const pad = (n: number) => n.toString().padStart(2, "0");
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

  // Same calendar day → show time only
  if (
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  ) {
    return time;
  }

  // Same year → show month-day + time
  if (d.getFullYear() === now.getFullYear()) {
    return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${time}`;
  }

  // Different year → show full date + time
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${time}`;
}
