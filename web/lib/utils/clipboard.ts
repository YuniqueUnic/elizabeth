"use client";

/**
 * Copy text to clipboard using the modern Clipboard API.
 * Throws if the API is unavailable (non-HTTPS context without browser permission).
 * The deprecated `document.execCommand('copy')` fallback has been removed — it was
 * unreliable, showed browser warnings, and is not supported in any test environment.
 * All callers should handle the thrown error and show a user-facing message.
 */
export async function copyTextToClipboard(text: string): Promise<void> {
  // 1. Try modern clipboard API if available
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (err) {
      console.warn("navigator.clipboard.writeText failed, trying fallback", err);
    }
  }

  // 2. Try ClipboardItem write API if available
  if (typeof navigator !== "undefined" && navigator.clipboard?.write && typeof ClipboardItem !== "undefined") {
    try {
      const type = "text/plain";
      const blob = new Blob([text], { type });
      const data = [new ClipboardItem({ [type]: blob })];
      await navigator.clipboard.write(data);
      return;
    } catch (err) {
      console.warn("navigator.clipboard.write with ClipboardItem failed, trying fallback", err);
    }
  }

  // 3. Fallback for non-secure contexts (HTTP) or when Clipboard APIs fail / are blocked
  if (typeof document !== "undefined") {
    const textArea = document.createElement("textarea");
    textArea.value = text;
    // Avoid scrolling to bottom
    textArea.style.top = "0";
    textArea.style.left = "0";
    textArea.style.position = "fixed";
    textArea.style.opacity = "0";
    textArea.style.pointerEvents = "none";
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
    try {
      const successful = (document as any).execCommand("copy");
      if (!successful) {
        throw new Error("document.execCommand('copy') returned false");
      }
    } catch (err) {
      console.error("Fallback copy failed:", err);
      throw new Error("Clipboard copy not supported in this browser environment");
    } finally {
      document.body.removeChild(textArea);
    }
    return;
  }

  throw new Error("No clipboard API or document context available");
}
