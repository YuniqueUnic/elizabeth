import type { Editor } from "@tiptap/react";
import type { FileItem } from "@/lib/types";

// 辅助函数：获取 markdown 内容
export function getMarkdownFromEditor(editor: Editor | null): string {
  if (!editor) return "";
  try {
    return editor.getMarkdown?.() ||
      editor.storage.markdown?.manager?.serialize(editor.getJSON()) ||
      editor.getText();
  } catch (e) {
    console.error("Failed to serialize markdown:", e);
    return editor.getText();
  }
}

// 辅助函数：设置 markdown 内容
export function setMarkdownToEditor(editor: Editor, markdown: string): void {
  try {
    const json = editor.storage.markdown?.manager?.parse(markdown);
    if (json) {
      editor.commands.setContent(json);
    } else {
      editor.commands.setContent(markdown, { contentType: "markdown" });
    }
  } catch (e) {
    console.error("Failed to parse markdown:", e);
    editor.commands.setContent(markdown, { contentType: "markdown" });
  }
}

// 辅助函数：插入 markdown 内容
export function insertMarkdownToEditor(editor: Editor, markdown: string): void {
  try {
    const json = editor.storage.markdown?.manager?.parse(markdown);
    if (json) {
      editor.commands.insertContent(json.content || json);
    } else {
      editor.commands.insertContent(markdown, { contentType: "markdown" });
    }
  } catch (e) {
    console.error("Failed to parse and insert markdown:", e);
    editor.commands.insertContent(markdown, { contentType: "markdown" });
  }
}

// 辅助函数：在 Textarea 中应用 Markdown 语法
export function applyMarkdownSyntax(
  textarea: HTMLTextAreaElement,
  format: string,
  value: string,
  onChange: (value: string) => void
) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const selection = value.substring(start, end);
  let before = value.substring(0, start);
  let after = value.substring(end);
  let newSelection = selection;
  let cursorOffset = 0;

  switch (format) {
    case "bold":
      newSelection = `**${selection}**`;
      cursorOffset = 2;
      break;
    case "italic":
      newSelection = `*${selection}*`;
      cursorOffset = 1;
      break;
    case "strike":
      newSelection = `~~${selection}~~`;
      cursorOffset = 2;
      break;
    case "code":
      newSelection = `\`${selection}\``;
      cursorOffset = 1;
      break;
    case "codeBlock": {
      const prefix = before.length === 0 || before.endsWith("\n")
        ? "```\n"
        : "\n```\n";
      const suffix = after.length === 0 || after.startsWith("\n")
        ? "\n```"
        : "\n```\n";
      newSelection = `${prefix}${selection}${suffix}`;
      cursorOffset = prefix.length;
      break;
    }
    case "heading-1":
      before = `${before}# `;
      cursorOffset = 2;
      break;
    case "heading-2":
      before = `${before}## `;
      cursorOffset = 3;
      break;
    case "heading-3":
      before = `${before}### `;
      cursorOffset = 4;
      break;
    case "bulletList":
      before = `${before}- `;
      cursorOffset = 2;
      break;
    case "orderedList":
      before = `${before}1. `;
      cursorOffset = 3;
      break;
    case "blockquote":
      before = `${before}> `;
      cursorOffset = 2;
      break;
  }

  const newValue = before + newSelection + after;
  onChange(newValue);

  // 恢复焦点和光标位置
  requestAnimationFrame(() => {
    textarea.focus();
    if (selection) {
      textarea.setSelectionRange(start + cursorOffset, end + cursorOffset);
    } else {
      textarea.setSelectionRange(start + cursorOffset, start + cursorOffset);
    }
  });
}

export function isLikelyImageFile(file: File): boolean {
  if (file.type.startsWith("image/")) return true;
  return /\.(png|jpe?g|gif|webp|svg)$/i.test(file.name);
}

export function fileToRoomPath(roomName: string, file: FileItem): string {
  if (file.url) return file.url;
  return `/contents/${file.id}`;
}

export function buildMarkdownForFile(
  roomName: string,
  file: FileItem,
  original: File,
): string {
  const href = fileToRoomPath(roomName, file);
  const name = file.name || "";
  if (isLikelyImageFile(original)) {
    return `\n\n![${name}](${href})\n`;
  }
  return `\n\n[${name}](${href})\n`;
}
