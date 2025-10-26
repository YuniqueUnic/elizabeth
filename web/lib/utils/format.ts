// Utility functions for formatting

export function formatFileSize(bytes?: number): string {
  if (!bytes) return "未知大小"

  const kb = bytes / 1024
  const mb = kb / 1024
  const gb = mb / 1024

  if (gb >= 1) {
    return `${gb.toFixed(2)} GB`
  }
  if (mb >= 1) {
    return `${mb.toFixed(2)} MB`
  }
  if (kb >= 1) {
    return `${kb.toFixed(2)} KB`
  }
  return `${bytes} B`
}

export function formatDate(date: string | Date): string {
  const d = typeof date === "string" ? new Date(date) : date
  const now = new Date()
  const diff = now.getTime() - d.getTime()

  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 7) {
    return d.toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    })
  }

  if (days > 0) return `${days}天前`
  if (hours > 0) return `${hours}小时前`
  if (minutes > 0) return `${minutes}分钟前`
  return "刚刚"
}
