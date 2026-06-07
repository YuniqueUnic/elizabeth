"use client";

/**
 * Copy text with the modern Clipboard API when the browser permits it.
 *
 * Public HTTP origins are not secure contexts, so Chrome can reject these calls.
 * The legacy `execCommand("copy")` path is only a best-effort fallback; UI callers
 * must still catch failures and provide a manual-copy path.
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
  if (
    typeof navigator !== "undefined" &&
    navigator.clipboard?.write &&
    typeof ClipboardItem !== "undefined"
  ) {
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

  // 3. Best-effort fallback for older browsers and some non-secure contexts.
  if (typeof document !== "undefined") {
    const textArea = document.createElement("textarea");
    textArea.value = text;
    textArea.setAttribute("readonly", ""); // Prevent virtual keyboard on mobile

    // Avoid scrolling and keep it invisible but selectable
    textArea.style.position = "fixed";
    textArea.style.top = "0";
    textArea.style.left = "0";
    textArea.style.width = "2em";
    textArea.style.height = "2em";
    textArea.style.padding = "0";
    textArea.style.border = "none";
    textArea.style.outline = "none";
    textArea.style.boxShadow = "none";
    textArea.style.background = "transparent";
    textArea.style.opacity = "0";

    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
    textArea.setSelectionRange(0, 99999); // Mobile selection support

    // Modern selection fallback
    const range = document.createRange();
    range.selectNodeContents(textArea);
    const selection = window.getSelection();
    if (selection) {
      selection.removeAllRanges();
      selection.addRange(range);
    }

    try {
      const successful = document.execCommand("copy");
      if (!successful) {
        throw new Error("document.execCommand('copy') returned false");
      }
    } catch (err) {
      console.error("Fallback copy failed:", err);
      throw new Error("Clipboard copy not supported in this browser environment");
    } finally {
      if (selection) {
        selection.removeAllRanges(); // Clean up selection
      }
      document.body.removeChild(textArea);
    }
    return;
  }

  throw new Error("No clipboard API or document context available");
}
