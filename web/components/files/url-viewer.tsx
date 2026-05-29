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
import { useTranslations } from "next-intl";

interface UrlViewerProps {
  url: string;
  name: string;
  description?: string;
}

export function UrlViewer({ url, name, description }: UrlViewerProps) {
  const t = useTranslations("room");
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
                  {t("urlViewer.iframeWarning")}
                </p>
              </div>
            )}
            <Button
              variant={showPreview ? "default" : "ghost"}
              size="sm"
              onClick={handleTogglePreview}
              title={showPreview ? t("urlViewer.hidePreview") : t("urlViewer.preview")}
            >
              {showPreview
                ? <EyeOff className="h-4 w-4" />
                : <Eye className="h-4 w-4" />}
              <span className="ml-2 hidden sm:inline">
                {showPreview ? t("urlViewer.hidePreview") : t("urlViewer.preview")}
              </span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenInNewTab}
              title={t("urlViewer.newTab")}
            >
              <ExternalLink className="h-4 w-4" />
              <span className="ml-2 hidden sm:inline">{t("urlViewer.newTab")}</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleToggleExpand}
              title={isExpanded ? t("urlViewer.collapseDetails") : t("urlViewer.expandDetails")}
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
              {t("urlViewer.previewHint")}
              <br />
              {t("urlViewer.newTabHint")}
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
                {t("urlViewer.iframeLoadError")}
                <br />
                <br />
                {t("urlViewer.openInNewTabHint")}
              </AlertDescription>
            </Alert>
            <Button
              variant="default"
              className="mt-4"
              onClick={handleOpenInNewTab}
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              {t("urlViewer.openInNewTab")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
