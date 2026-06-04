// Utility functions for formatting

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
