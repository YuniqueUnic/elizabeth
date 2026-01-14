"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  AlertCircle,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  Eye,
  EyeOff,
} from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

interface UrlViewerProps {
  url: string;
  name: string;
  description?: string;
}

export function UrlViewer({ url, name, description }: UrlViewerProps) {
  const [showPreview, setShowPreview] = useState(false);
  const [iframeError, setIframeError] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  const handleOpenInNewTab = () => {
    window.open(url, "_blank", "noopener,noreferrer");
  };

  const handleTogglePreview = () => {
    setShowPreview((prev) => !prev);
    setIframeError(false);
  };

  const handleToggleExpand = () => {
    setIsExpanded((prev) => !prev);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Compact Toolbar */}
      <div className="border-b bg-muted/30">
        {/* Main Toolbar Row */}
        <div className="flex items-center justify-between gap-2 px-4">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            <span className="font-medium truncate">{name}</span>
            {!isExpanded && (
              <a
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-primary hover:underline truncate max-w-xs"
                title={url}
              >
                {url}
              </a>
            )}
          </div>

          <div className="flex items-center gap-1">
            {showPreview && !iframeError && (
              <div className="flex items-center gap-1">
                <AlertCircle className="h-4 w-4" />
                <p className="text-sm text-muted-foreground pr-3">
                  某些网站可能不允许在 iframe
                  中显示。如果预览失败，请使用“新标签页打开”
                </p>
              </div>
            )}
            <Button
              variant={showPreview ? "default" : "ghost"}
              size="sm"
              onClick={handleTogglePreview}
              title={showPreview ? "隐藏预览" : "显示预览"}
            >
              {showPreview
                ? <EyeOff className="h-4 w-4" />
                : <Eye className="h-4 w-4" />}
              <span className="ml-2 hidden sm:inline">
                {showPreview ? "隐藏预览" : "预览"}
              </span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenInNewTab}
              title="在新标签页打开"
            >
              <ExternalLink className="h-4 w-4" />
              <span className="ml-2 hidden sm:inline">新标签页</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleToggleExpand}
              title={isExpanded ? "收起详情" : "展开详情"}
            >
              {isExpanded
                ? <ChevronUp className="h-4 w-4" />
                : <ChevronDown className="h-4 w-4" />}
            </Button>
          </div>
        </div>

        {/* Expanded Details */}
        {isExpanded && (
          <div className="px-4 pb-3 pt-1 border-t bg-muted/10">
            {description && (
              <p className="text-sm text-muted-foreground mb-2">
                {description}
              </p>
            )}
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">URL:</span>
              <a
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline break-all"
              >
                {url}
              </a>
            </div>
          </div>
        )}
      </div>

      {/* Preview Area */}
      <div className="flex-1 overflow-auto">
        {!showPreview && (
          <div className="flex flex-col items-center justify-center h-full p-8 text-muted-foreground">
            <ExternalLink className="h-12 w-12 mb-4 opacity-50" />
            <p className="text-center">
              点击“预览”按钮查看链接内容
              <br />
              或点击“新标签页打开”在浏览器中打开
            </p>
          </div>
        )}

        {showPreview && !iframeError && (
          <div className="h-full p-4">
            <iframe
              src={url}
              className="w-full h-[calc(100%-5rem)] border rounded-lg"
              sandbox="allow-scripts allow-same-origin allow-popups allow-forms"
              onError={() => setIframeError(true)}
              title={name}
            />
          </div>
        )}

        {showPreview && iframeError && (
          <div className="flex flex-col items-center justify-center h-full p-8">
            <Alert variant="destructive" className="max-w-md">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                无法在预览中加载此链接。该网站可能不允许嵌入显示。
                <br />
                <br />
                请点击“新标签页打开”按钮在浏览器中查看。
              </AlertDescription>
            </Alert>
            <Button
              variant="default"
              className="mt-4"
              onClick={handleOpenInNewTab}
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              新标签页打开
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
