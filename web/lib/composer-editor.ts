import type { MDXEditorMethods } from "@mdxeditor/editor";

let activeComposerEditor: MDXEditorMethods | null = null;

export function registerComposerEditor(editor: MDXEditorMethods | null) {
  activeComposerEditor = editor;
  if (typeof window !== "undefined" && process.env.NODE_ENV !== "production") {
    (window as any).__ELIZABETH_ACTIVE_EDITOR__ = editor;
  }
}

export function unregisterComposerEditor(editor: MDXEditorMethods | null) {
  if (!editor) return;
  if (activeComposerEditor === editor) {
    activeComposerEditor = null;
  }
}

export function insertMarkdownIntoComposer(markdown: string): boolean {
  if (!activeComposerEditor) return false;
  activeComposerEditor.focus(() => {
    activeComposerEditor?.insertMarkdown(markdown);
  });
  return true;
}
