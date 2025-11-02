"use client";

import { useEffect, useState } from "react";
import { codeToHtml, type BundledLanguage, type BundledTheme } from "shiki";

interface CodeBlockProps {
  code: string;
  language: string;
  theme?: "dark" | "light";
  showLineNumbers?: boolean;
  className?: string;
}

export function CodeBlock({
  code,
  language,
  theme = "dark",
  showLineNumbers = true,
  className = "",
}: CodeBlockProps) {
  const [html, setHtml] = useState<string>("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const highlight = async () => {
      try {
        setLoading(true);

        // Map theme to Shiki theme names
        const shikiTheme: BundledTheme = theme === "dark"
          ? "vitesse-dark"
          : "vitesse-light";

        // Normalize language name
        const normalizedLang = normalizeLanguage(language);

        const highlighted = await codeToHtml(code, {
          lang: normalizedLang as BundledLanguage,
          theme: shikiTheme,
          structure: "inline",
          decorations: showLineNumbers ? undefined : [],
        });

        if (!cancelled) {
          setHtml(highlighted);
          setLoading(false);
        }
      } catch (error) {
        console.error("Failed to highlight code:", error);
        if (!cancelled) {
          // Fallback to plain text
          setHtml(`<pre><code>${escapeHtml(code)}</code></pre>`);
          setLoading(false);
        }
      }
    };

    highlight();

    return () => {
      cancelled = true;
    };
  }, [code, language, theme, showLineNumbers]);

  if (loading) {
    return (
      <div className={`rounded-lg bg-muted p-4 ${className}`}>
        <div className="animate-pulse space-y-2">
          <div className="h-4 bg-muted-foreground/20 rounded w-3/4"></div>
          <div className="h-4 bg-muted-foreground/20 rounded w-1/2"></div>
          <div className="h-4 bg-muted-foreground/20 rounded w-5/6"></div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`shiki-code-block ${className}`}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

// Normalize language names to match Shiki's bundled languages
function normalizeLanguage(lang: string): string {
  const langMap: Record<string, string> = {
    js: "javascript",
    jsx: "jsx",
    ts: "typescript",
    tsx: "tsx",
    py: "python",
    rb: "ruby",
    rs: "rust",
    go: "go",
    java: "java",
    cpp: "cpp",
    c: "c",
    cs: "csharp",
    php: "php",
    swift: "swift",
    kt: "kotlin",
    kts: "kotlin",
    scala: "scala",
    dart: "dart",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    fish: "bash",
    ps1: "powershell",
    html: "html",
    htm: "html",
    css: "css",
    scss: "scss",
    sass: "sass",
    less: "less",
    xml: "xml",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    sql: "sql",
    graphql: "graphql",
    gql: "graphql",
    proto: "protobuf",
    dockerfile: "dockerfile",
    makefile: "makefile",
    cmake: "cmake",
    nginx: "nginx",
    conf: "nginx",
    md: "markdown",
    markdown: "markdown",
    text: "text",
    txt: "text",
  };

  return langMap[lang.toLowerCase()] || lang;
}

// Escape HTML to prevent XSS
function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  };
  return text.replace(/[&<>"']/g, (m) => map[m]);
}
