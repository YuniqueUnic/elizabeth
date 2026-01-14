"use client";

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeHighlighter } from "./code-highlighter";
import { useAppStore } from "@/lib/store";
import { API_BASE_URL } from "@/lib/config";
import { getRoomTokenString } from "@/lib/utils/api";

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  const currentRoomId = useAppStore((state) => state.currentRoomId);

  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ className, children, ...props }) {
          const match = /language-(\w+)/.exec(className || "");
          const lang = match ? match[1] : "";
          const codeString = String(children).replace(/\n$/, "");

          const isInlineCode = (!className && !codeString.includes("\n")) ||
            (props as any).inline === true;

          if (isInlineCode) {
            return (
              <code
                className="px-1.5 py-0.5 rounded bg-muted text-sm font-mono"
                {...props}
              >
                {codeString}
              </code>
            );
          }

          return <CodeHighlighter code={codeString} language={lang} inline={false} />;
        },
        a({ children, href, ...props }) {
          let resolvedHref = typeof href === "string" ? href : undefined;

          if (typeof href === "string" && href.startsWith("/")) {
            const token = getRoomTokenString(currentRoomId);
            const hasApiPrefix = href.startsWith(`${API_BASE_URL}/`);
            const roomPath = href.startsWith("/rooms/")
              ? `${API_BASE_URL}${href}`
              : href;
            const finalPath = hasApiPrefix ? href : roomPath;

            if (token) {
              const url = new URL(finalPath, window.location.origin);
              url.searchParams.set("token", token);
              resolvedHref = `${url.pathname}${url.search}${url.hash}`;
            } else {
              resolvedHref = finalPath;
            }
          }

          return (
            <a
              {...props}
              href={resolvedHref}
              className="text-primary hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              {children}
            </a>
          );
        },
        img({ alt, src, ...props }) {
          let resolvedSrc = typeof src === "string" ? src : undefined;

          if (typeof src === "string" && src.startsWith("/")) {
            const token = getRoomTokenString(currentRoomId);
            const hasApiPrefix = src.startsWith(`${API_BASE_URL}/`);
            const roomPath = src.startsWith("/rooms/")
              ? `${API_BASE_URL}${src}`
              : src;
            const finalPath = hasApiPrefix ? src : roomPath;

            if (token) {
              const url = new URL(finalPath, window.location.origin);
              url.searchParams.set("token", token);
              resolvedSrc = `${url.pathname}${url.search}${url.hash}`;
            } else {
              resolvedSrc = finalPath;
            }
          }

          return (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              {...props}
              src={resolvedSrc}
              alt={alt ?? ""}
              className="max-w-full rounded-md border border-border"
              loading="lazy"
            />
          );
        },
        table({ children, ...props }) {
          return (
            <div className="overflow-x-auto my-4">
              <table className="min-w-full divide-y divide-border" {...props}>
                {children}
              </table>
            </div>
          );
        },
        ul({ children, ...props }) {
          return (
            <ul className="list-disc list-inside space-y-1 my-2" {...props}>
              {children}
            </ul>
          );
        },
        ol({ children, ...props }) {
          return (
            <ol className="list-decimal list-inside space-y-1 my-2" {...props}>
              {children}
            </ol>
          );
        },
        blockquote({ children, ...props }) {
          return (
            <blockquote
              className="border-l-4 border-primary/30 pl-4 italic my-4 text-muted-foreground"
              {...props}
            >
              {children}
            </blockquote>
          );
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
