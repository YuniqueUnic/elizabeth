"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  AlertCircle,
  Check,
  ChevronDown,
  Code2,
  Copy,
  Eye,
  Moon,
  Sun,
} from "lucide-react";
import { api } from "@/lib/utils/api";
import { Button } from "@/components/ui/button";
import { useToast } from "@/hooks/use-toast";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { CodeBlock } from "@/components/ui/code-block";

interface FileContentPreviewProps {
  fileUrl: string;
  fileName: string;
  mimeType?: string;
  roomName: string;
  onFullscreenToggle?: (isFullscreen: boolean) => void;
}

// Detect file type from extension
function getFileType(fileName: string): string {
  const ext = fileName.split(".").pop()?.toLowerCase() || "";

  // Markdown files
  if (["md", "markdown"].includes(ext)) return "markdown";

  // Code files
  const codeExtensions = [
    "js",
    "jsx",
    "ts",
    "tsx",
    "py",
    "java",
    "c",
    "cpp",
    "cs",
    "go",
    "rs",
    "rb",
    "php",
    "swift",
    "kt",
    "scala",
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "html",
    "css",
    "scss",
    "sass",
    "less",
    "xml",
    "json",
    "yaml",
    "yml",
    "toml",
    "sql",
    "graphql",
    "proto",
    "dockerfile",
    "makefile",
    "cmake",
  ];
  if (codeExtensions.includes(ext)) return "code";

  // Plain text files
  const textExtensions = [
    "txt",
    "log",
    "csv",
    "tsv",
    "ini",
    "conf",
    "cfg",
    "env",
  ];
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
    cc: "cpp",
    cxx: "cpp",
    h: "cpp",
    hpp: "cpp",
    cs: "csharp",
    go: "go",
    rs: "rust",
    php: "php",
    swift: "swift",
    kt: "kotlin",
    kts: "kotlin",
    scala: "scala",
    dart: "dart", // ✅ ADD: Dart support
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
  };

  return languageMap[ext] || "text";
}

// Common programming languages for syntax highlighting
const SUPPORTED_LANGUAGES = [
  { value: "auto", label: "自动检测" },
  { value: "c", label: "C" },
  { value: "go", label: "Go" },
  { value: "rust", label: "Rust" },
  { value: "bash", label: "Bash/Shell" },
  { value: "dart", label: "Dart" },
  { value: "javascript", label: "JavaScript" },
  { value: "typescript", label: "TypeScript" },
  { value: "jsx", label: "React JSX" },
  { value: "tsx", label: "React TSX" },
  { value: "python", label: "Python" },
  { value: "java", label: "Java" },
  { value: "cpp", label: "C++" },
  { value: "csharp", label: "C#" },
  { value: "ruby", label: "Ruby" },
  { value: "php", label: "PHP" },
  { value: "swift", label: "Swift" },
  { value: "kotlin", label: "Kotlin" },
  { value: "scala", label: "Scala" },
  { value: "powershell", label: "PowerShell" },
  { value: "sql", label: "SQL" },
  { value: "json", label: "JSON" },
  { value: "yaml", label: "YAML" },
  { value: "toml", label: "TOML" },
  { value: "xml", label: "XML" },
  { value: "html", label: "HTML" },
  { value: "css", label: "CSS" },
  { value: "scss", label: "SCSS" },
  { value: "less", label: "Less" },
  { value: "markdown", label: "Markdown" },
  { value: "dockerfile", label: "Dockerfile" },
  { value: "nginx", label: "Nginx" },
  { value: "graphql", label: "GraphQL" },
  { value: "text", label: "纯文本" },
];

export function FileContentPreview(
  { fileUrl, fileName }: FileContentPreviewProps,
) {
  const { toast } = useToast();
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [darkTheme, setDarkTheme] = useState(true);
  const [markdownPreviewMode, setMarkdownPreviewMode] = useState(true); // true = preview, false = code

  const fileType = getFileType(fileName);
  const detectedLanguage = getLanguage(fileName);
  const [selectedLanguage, setSelectedLanguage] = useState<string>("auto");

  // Use selected language or fall back to detected language
  const language = selectedLanguage === "auto"
    ? detectedLanguage
    : selectedLanguage;

  useEffect(() => {
    const fetchContent = async () => {
      try {
        setLoading(true);
        setError(null);

        // ✅ FIX: Use API client to automatically add token
        // The fileUrl is a relative path like /api/v1/rooms/{roomName}/contents/{id}
        // We need to fetch it with authentication
        const response = await api.getRaw(fileUrl);

        if (!response.ok) {
          throw new Error(`Failed to fetch file: ${response.statusText}`);
        }

        const text = await response.text();
        setContent(text);
      } catch (err) {
        console.error("Failed to load file content:", err);
        setError(
          err instanceof Error ? err.message : "Failed to load file content",
        );
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

  // Handle copy to clipboard
  const handleCopy = async () => {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast({
        title: "已复制",
        description: "内容已复制到剪贴板",
      });
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      toast({
        title: "复制失败",
        description: "无法复制内容",
        variant: "destructive",
      });
    }
  };

  // Toolbar component
  const Toolbar = () => (
    <div className="flex items-center justify-between gap-2 px-4 py-2 border-b bg-muted/30">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-muted-foreground">
          {fileName}
        </span>
        {fileType === "code" && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="outline"
                size="sm"
                className="h-6 text-xs px-2 py-0"
              >
                {SUPPORTED_LANGUAGES.find((l) => l.value === selectedLanguage)
                  ?.label || language}
                <ChevronDown className="ml-1 h-3 w-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="start"
              className="max-h-[300px] overflow-y-auto"
            >
              {SUPPORTED_LANGUAGES.map((lang) => (
                <DropdownMenuItem
                  key={lang.value}
                  onClick={() => setSelectedLanguage(lang.value)}
                  className="cursor-pointer"
                >
                  {selectedLanguage === lang.value && (
                    <Check className="mr-2 h-4 w-4" />
                  )}
                  <span
                    className={selectedLanguage !== lang.value ? "ml-6" : ""}
                  >
                    {lang.label}
                  </span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
      <div className="flex items-center gap-1">
        {fileType === "markdown" && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setMarkdownPreviewMode(!markdownPreviewMode)}
            title={markdownPreviewMode ? "查看代码" : "预览渲染"}
          >
            {markdownPreviewMode
              ? <Code2 className="h-4 w-4" />
              : <Eye className="h-4 w-4" />}
          </Button>
        )}
        {(fileType === "code" || fileType === "text" ||
          (fileType === "markdown" && !markdownPreviewMode)) && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDarkTheme(!darkTheme)}
            title="切换主题"
          >
            {darkTheme ? <Moon /> : <Sun />}
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          title="复制内容"
        >
          {copied
            ? <Check className="h-4 w-4" />
            : <Copy className="h-4 w-4" />}
        </Button>
      </div>
    </div>
  );

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
      <div className="flex flex-col h-full">
        <Toolbar />
        {markdownPreviewMode
          ? (
            // Preview mode: Render Markdown
            <div className="prose prose-sm dark:prose-invert max-w-none p-6 overflow-auto flex-1">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  code({ className, children, ...props }) {
                    const match = /language-(\w+)/.exec(className || "");
                    const isInline = !match;
                    return !isInline
                      ? (
                        <CodeBlock
                          code={String(children).replace(/\n$/, "")}
                          language={match[1]}
                          theme={darkTheme ? "dark" : "light"}
                          showLineNumbers={false}
                        />
                      )
                      : (
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
          )
          : (
            // Code mode: Show raw Markdown with syntax highlighting
            <div className="p-4 overflow-auto flex-1">
              <CodeBlock
                code={content}
                language="markdown"
                theme={darkTheme ? "dark" : "light"}
                showLineNumbers={true}
              />
            </div>
          )}
      </div>
    );
  }

  // Render code with syntax highlighting
  if (fileType === "code") {
    return (
      <div className="flex flex-col h-full">
        <Toolbar />
        <div className="p-4 overflow-auto flex-1">
          <CodeBlock
            code={content}
            language={language}
            theme={darkTheme ? "dark" : "light"}
            showLineNumbers={true}
          />
        </div>
      </div>
    );
  }

  // Render plain text
  if (fileType === "text") {
    return (
      <div className="flex flex-col h-full">
        <Toolbar />
        <div className="p-6 overflow-auto flex-1">
          <pre
            className={`whitespace-pre-wrap font-mono text-sm p-4 rounded-lg ${
              darkTheme
                ? "bg-gray-900 text-gray-100"
                : "bg-gray-100 text-gray-900"
            }`}
          >
            {content}
          </pre>
        </div>
      </div>
    );
  }

  return null;
}
