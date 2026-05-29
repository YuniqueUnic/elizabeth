"use client";

import type React from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { useAppStore } from "@/lib/store";
import { useTranslations } from "next-intl";

export function SettingsDialog({ children }: { children: React.ReactNode }) {
  const t = useTranslations("settings");
  const {
    sendOnEnter,
    setSendOnEnter,
    includeMetadataInCopy,
    setIncludeMetadataInCopy,
    includeMetadataInDownload,
    setIncludeMetadataInDownload,
    editorFontSize,
    setEditorFontSize,
    toolbarButtonSize,
    setToolbarButtonSize,
    messageFontSize,
    setMessageFontSize,
    useHeti,
    setUseHeti,
    showDeleteConfirmation,
    setShowDeleteConfirmation,
    autoScroll,
    setAutoScroll,
  } = useAppStore();

  return (
    <Dialog>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Send on Enter Setting */}
          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="send-on-enter">{t("sendOnEnter.label")}</Label>
              <p className="text-sm text-muted-foreground">
                {sendOnEnter
                  ? t("sendOnEnter.enabled")
                  : t("sendOnEnter.disabled")}
              </p>
            </div>
            <Switch
              id="send-on-enter"
              checked={sendOnEnter}
              onCheckedChange={setSendOnEnter}
            />
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="include-metadata-copy">
                {t("includeMetadataInCopy.label")}
              </Label>
              <p className="text-sm text-muted-foreground">
                {t("includeMetadataInCopy.description")}
              </p>
            </div>
            <Switch
              id="include-metadata-copy"
              checked={includeMetadataInCopy}
              onCheckedChange={setIncludeMetadataInCopy}
            />
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="include-metadata-download">
                {t("includeMetadataInDownload.label")}
              </Label>
              <p className="text-sm text-muted-foreground">
                {t("includeMetadataInDownload.description")}
              </p>
            </div>
            <Switch
              id="include-metadata-download"
              checked={includeMetadataInDownload}
              onCheckedChange={setIncludeMetadataInDownload}
            />
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="use-heti">{t("useHeti.label")}</Label>
              <p className="text-sm text-muted-foreground">
                {t("useHeti.description")}
              </p>
            </div>
            <Switch
              id="use-heti"
              checked={useHeti}
              onCheckedChange={setUseHeti}
            />
          </div>

          {/* Show Delete Confirmation Setting */}
          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="delete-confirmation">{t("showDeleteConfirmation.label")}</Label>
              <p className="text-sm text-muted-foreground">
                {t("showDeleteConfirmation.description")}
              </p>
            </div>
            <Switch
              id="delete-confirmation"
              checked={showDeleteConfirmation}
              onCheckedChange={setShowDeleteConfirmation}
            />
          </div>

          {/* Auto-scroll Setting */}
          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="auto-scroll">{t("autoScroll.label")}</Label>
              <p className="text-sm text-muted-foreground">
                {t("autoScroll.description")}
              </p>
            </div>
            <Switch
              id="auto-scroll"
              checked={autoScroll}
              onCheckedChange={setAutoScroll}
            />
          </div>

          {/* Editor Font Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="editor-font-size">{t("editorFontSize.label")}</Label>
            <div className="flex items-center gap-4">
              <Slider
                id="editor-font-size"
                min={4}
                max={64}
                step={1}
                value={[editorFontSize]}
                onValueChange={([value]) => setEditorFontSize(value)}
                className="flex-1"
              />
              <Input
                type="number"
                min={4}
                max={64}
                value={editorFontSize}
                onChange={(e) => {
                  const value = parseInt(e.target.value, 10);
                  if (!isNaN(value) && value >= 4 && value <= 64) {
                    setEditorFontSize(value);
                  }
                }}
                className="w-20"
              />
            </div>
            <p className="text-sm text-muted-foreground">
              {t("editorFontSize.description")}
            </p>
          </div>

          {/* Toolbar Button Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="toolbar-button-size">{t("toolbarButtonSize.label")}</Label>
            <div className="flex items-center gap-4">
              <Slider
                id="toolbar-button-size"
                min={20}
                max={36}
                step={2}
                value={[toolbarButtonSize]}
                onValueChange={([value]) => setToolbarButtonSize(value)}
                className="flex-1"
              />
              <Input
                type="number"
                min={20}
                max={36}
                step={2}
                value={toolbarButtonSize}
                onChange={(e) => {
                  const value = parseInt(e.target.value, 10);
                  if (!isNaN(value) && value >= 20 && value <= 36) {
                    setToolbarButtonSize(value);
                  }
                }}
                className="w-20"
              />
            </div>
            <p className="text-sm text-muted-foreground">
              {t("toolbarButtonSize.description")}
            </p>
          </div>

          {/* Message Font Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="message-font-size">{t("messageFontSize.label")}</Label>
            <div className="flex items-center gap-4">
              <Slider
                id="message-font-size"
                min={4}
                max={64}
                step={1}
                value={[messageFontSize]}
                onValueChange={([value]) => setMessageFontSize(value)}
                className="flex-1"
              />
              <Input
                type="number"
                min={4}
                max={64}
                value={messageFontSize}
                onChange={(e) => {
                  const value = parseInt(e.target.value, 10);
                  if (!isNaN(value) && value >= 4 && value <= 64) {
                    setMessageFontSize(value);
                  }
                }}
                className="w-20"
              />
            </div>
            <p className="text-sm text-muted-foreground">
              {t("messageFontSize.description")}
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
