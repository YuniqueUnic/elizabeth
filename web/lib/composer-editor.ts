import type { MinimalTiptapEditorMethods } from "@/components/chat/minimal-tiptap-editor";

let activeComposerEditor: MinimalTiptapEditorMethods | null = null;

export function registerComposerEditor(editor: MinimalTiptapEditorMethods | null) {
  activeComposerEditor = editor;
  if (typeof window !== "undefined" && process.env.NODE_ENV !== "production") {
    (window as any).__ELIZABETH_ACTIVE_EDITOR__ = editor;
  }
}

export function unregisterComposerEditor(editor: MinimalTiptapEditorMethods | null) {
  if (!editor) return;
  if (activeComposerEditor === editor) {
    activeComposerEditor = null;
  }
}

export function insertMarkdownIntoComposer(markdown: string): boolean {
  if (!activeComposerEditor) return false;
  activeComposerEditor.focus({ position: "end" });
  activeComposerEditor.insertMarkdown(markdown);
  return true;
}
