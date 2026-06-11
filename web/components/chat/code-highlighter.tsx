"use client";

import { useEffect, useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/lib/store";
import { type BundledLanguage, type BundledTheme, codeToHtml } from "shiki";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import {
  getCodeBlockLanguageLabel,
  normalizeCodeBlockLanguage,
} from "./code-block-language";

interface CodeHighlighterProps {
  code: string;
  language?: string;
  inline?: boolean;
}

export function CodeHighlighter(
  { code, language, inline }: CodeHighlighterProps,
) {
  const [copied, setCopied] = useState(false);
  const [manualCopyValue, setManualCopyValue] = useState("");
  const [highlighted, setHighlighted] = useState<string>("");
  const theme = useAppStore((state) => state.theme);
  const codeLanguage = normalizeCodeBlockLanguage(language);
  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">(
    "light",
  );

  // 解析主题：如果是 system，则根据系统偏好设置
  useEffect(() => {
    if (theme === "system") {
      const mediaQuery = window.matchMedia(
        "(prefers-color-scheme: dark)",
      );
      const updateTheme = () => {
        setResolvedTheme(mediaQuery.matches ? "dark" : "light");
      };

      updateTheme();
      mediaQuery.addEventListener("change", updateTheme);

      return () => mediaQuery.removeEventListener("change", updateTheme);
    } else {
      setResolvedTheme(theme);
    }
  }, [theme]);

  useEffect(() => {
    if (inline) return;

    let cancelled = false;

    const highlightCode = async () => {
      try {
        const shikiTheme: BundledTheme =
          resolvedTheme === "dark" ? "github-dark" : "github-light";
        const html = await codeToHtml(code, {
          lang: codeLanguage as BundledLanguage,
          theme: shikiTheme,
          transformers: [{
            line(node, line) {
              node.properties["data-line"] = line;
              this.addClassToHast(node, "line");
            },
            pre(node) {
              this.addClassToHast(node, "shiki-pre");
            },
            code(node) {
              this.addClassToHast(node, "shiki-code");
            },
          }],
        });
        if (!cancelled) setHighlighted(html);
      } catch (error) {
        console.error("Error highlighting code:", error);
        if (!cancelled) setHighlighted(buildFallbackCodeHtml(code));
      }
    };

    highlightCode();

    return () => {
      cancelled = true;
    };
  }, [code, codeLanguage, resolvedTheme, inline]);

  const handleCopy = async () => {
    try {
      await copyTextToClipboard(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy code:", err);
      setManualCopyValue(code);
    }
  };

  if (inline) {
    return (
      <code className="px-1.5 py-0.5 rounded bg-muted text-sm font-mono">
        {code}
      </code>
    );
  }

  return (
    <>
      <div
        className="relative group my-4 rounded-lg border bg-muted/50 w-full max-w-full overflow-hidden"
        data-language={codeLanguage}
        data-testid="shiki-code-block"
      >
        <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
          <span className="text-xs font-medium text-muted-foreground">
            {getCodeBlockLanguageLabel(codeLanguage)}
          </span>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={handleCopy}
          >
            {copied
              ? <Check className="h-3 w-3" />
              : <Copy className="h-3 w-3" />}
          </Button>
        </div>
        {highlighted
          ? (
            <div
              className="shiki-wrapper overflow-hidden [&>pre]:m-0 [&>pre]:border-0"
              dangerouslySetInnerHTML={{ __html: highlighted }}
            />
          )
          : (
            <pre className="overflow-x-auto p-4">
              <code className="text-sm font-mono leading-relaxed block">{code}</code>
            </pre>
          )}
      </div>
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setManualCopyValue("");
        }}
      />
    </>
  );
}

function buildFallbackCodeHtml(code: string): string {
  const lines = escapeHtml(code).split("\n");
  const lineHtml = lines
    .map((line, index) =>
      `<span class="line" data-line="${index + 1}">${line}</span>`)
    .join("\n");

  return `<pre class="shiki-pre"><code class="shiki-code">${lineHtml}</code></pre>`;
}

function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  };

  return text.replace(/[&<>"']/g, (value) => map[value]);
}
