"use client";

/**
 * Copy text to clipboard using the modern Clipboard API.
 * Throws if the API is unavailable (non-HTTPS context without browser permission).
 * The deprecated `document.execCommand('copy')` fallback has been removed — it was
 * unreliable, showed browser warnings, and is not supported in any test environment.
 * All callers should handle the thrown error and show a user-facing message.
 */
export async function copyTextToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  // Final fallback: selection + clipboard item API (still modern, no execCommand)
  const type = "text/plain";
  const blob = new Blob([text], { type });
  const data = [new ClipboardItem({ [type]: blob })];
  await navigator.clipboard.write(data);
}
