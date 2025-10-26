"use client";

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeHighlighter } from "./code-highlighter";

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="prose prose-sm dark:prose-invert max-w-none">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code({ node, inline, className, children, ...props }) {
            const match = /language-(\w+)/.exec(className || "");
            const lang = match ? match[1] : "";
            const codeString = String(children).replace(/\n$/, "");

            // 内联代码的判断：
            // 1. inline 参数为 true
            // 2. 或者没有语言标识且没有换行符
            const isInlineCode = inline === true ||
              (!className && !codeString.includes("\n"));

            // 内联代码：直接返回 <code> 标签（可以在 <p> 内）
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

            // 代码块：返回完整的高亮组件（块级元素）
            return (
              <CodeHighlighter
                code={codeString}
                language={lang}
                inline={false}
              />
            );
          },
          a({ node, children, ...props }) {
            return (
              <a
                {...props}
                className="text-primary hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                {children}
              </a>
            );
          },
          table({ node, children, ...props }) {
            return (
              <div className="overflow-x-auto my-4">
                <table className="min-w-full divide-y divide-border" {...props}>
                  {children}
                </table>
              </div>
            );
          },
          ul({ node, children, ...props }) {
            return (
              <ul className="list-disc list-inside space-y-1 my-2" {...props}>
                {children}
              </ul>
            );
          },
          ol({ node, children, ...props }) {
            return (
              <ol
                className="list-decimal list-inside space-y-1 my-2"
                {...props}
              >
                {children}
              </ol>
            );
          },
          blockquote({ node, children, ...props }) {
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
    </div>
  );
}
