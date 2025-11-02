"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";

interface FileContentPreviewProps {
  fileUrl: string;
  fileName: string;
  mimeType?: string;
}

// Detect file type from extension
function getFileType(fileName: string): string {
  const ext = fileName.split(".").pop()?.toLowerCase() || "";

  // Markdown files
  if (["md", "markdown"].includes(ext)) return "markdown";

  // Code files
  const codeExtensions = [
    "js", "jsx", "ts", "tsx", "py", "java", "c", "cpp", "cs", "go", "rs", "rb",
    "php", "swift", "kt", "scala", "sh", "bash", "zsh", "fish", "ps1",
    "html", "css", "scss", "sass", "less", "xml", "json", "yaml", "yml", "toml",
    "sql", "graphql", "proto", "dockerfile", "makefile", "cmake",
  ];
  if (codeExtensions.includes(ext)) return "code";

  // Plain text files
  const textExtensions = ["txt", "log", "csv", "tsv", "ini", "conf", "cfg", "env"];
  if (textExtensions.includes(ext)) return "text";

  return "unknown";
}

// Get language for syntax highlighting
function getLanguage(fileName: string): string {
  const ext = fileName.split(".").pop()?.toLowerCase() || "";

  const languageMap: Record<string, string> = {
    js: "javascript",
    jsx: "jsx",
    ts: "typescript",
    tsx: "tsx",
    py: "python",
    rb: "ruby",
    java: "java",
    c: "c",
    cpp: "cpp",
    cs: "csharp",
    go: "go",
    rs: "rust",
    php: "php",
    swift: "swift",
    kt: "kotlin",
    scala: "scala",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    fish: "bash",
    ps1: "powershell",
    html: "html",
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
    proto: "protobuf",
    dockerfile: "dockerfile",
    makefile: "makefile",
    cmake: "cmake",
  };

  return languageMap[ext] || "text";
}

export function FileContentPreview({ fileUrl, fileName, mimeType }: FileContentPreviewProps) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fileType = getFileType(fileName);
  const language = getLanguage(fileName);

  useEffect(() => {
    const fetchContent = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await fetch(fileUrl);
        if (!response.ok) {
          throw new Error(`Failed to fetch file: ${response.statusText}`);
        }

        const text = await response.text();
        setContent(text);
      } catch (err) {
        console.error("Failed to load file content:", err);
        setError(err instanceof Error ? err.message : "Failed to load file content");
      } finally {
        setLoading(false);
      }
    };

    if (fileType !== "unknown") {
      fetchContent();
    } else {
      setLoading(false);
    }
  }, [fileUrl, fileType]);

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <LoadingSpinner className="h-8 w-8" />
      </div>
    );
  }

  if (error) {
    return (
      <Alert variant="destructive" className="m-4">
        <AlertCircle className="h-4 w-4" />
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (fileType === "unknown") {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
        <p>无法预览此文件类型</p>
        <p className="text-sm mt-2">请下载文件以查看内容</p>
      </div>
    );
  }

  if (!content) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
        <p>文件内容为空</p>
      </div>
    );
  }

  // Render Markdown
  if (fileType === "markdown") {
    return (
      <div className="prose prose-sm dark:prose-invert max-w-none p-6">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            code({ node, inline, className, children, ...props }) {
              const match = /language-(\w+)/.exec(className || "");
              return !inline && match ? (
                <SyntaxHighlighter
                  style={vscDarkPlus}
                  language={match[1]}
                  PreTag="div"
                  {...props}
                >
                  {String(children).replace(/\n$/, "")}
                </SyntaxHighlighter>
              ) : (
                <code className={className} {...props}>
                  {children}
                </code>
              );
            },
          }}
        >
          {content}
        </ReactMarkdown>
      </div>
    );
  }

  // Render code with syntax highlighting
  if (fileType === "code") {
    return (
      <div className="p-4">
        <SyntaxHighlighter
          language={language}
          style={vscDarkPlus}
          showLineNumbers
          wrapLines
          customStyle={{
            margin: 0,
            borderRadius: "0.5rem",
            fontSize: "0.875rem",
          }}
        >
          {content}
        </SyntaxHighlighter>
      </div>
    );
  }

  // Render plain text
  if (fileType === "text") {
    return (
      <div className="p-6">
        <pre className="whitespace-pre-wrap font-mono text-sm bg-muted p-4 rounded-lg overflow-auto max-h-[60vh]">
          {content}
        </pre>
      </div>
    );
  }

  return null;
}
