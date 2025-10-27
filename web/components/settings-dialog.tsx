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

export function SettingsDialog({ children }: { children: React.ReactNode }) {
  const {
    sendOnEnter,
    setSendOnEnter,
    includeMetadataInExport,
    setIncludeMetadataInExport,
    editorFontSize,
    setEditorFontSize,
    toolbarButtonSize,
    setToolbarButtonSize,
    messageFontSize,
    setMessageFontSize,
    useHeti,
    setUseHeti,
  } = useAppStore();

  return (
    <Dialog>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>设置</DialogTitle>
          <DialogDescription>配置应用程序的行为和偏好设置</DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Send on Enter Setting */}
          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="send-on-enter">按 Enter 键发送消息</Label>
              <p className="text-sm text-muted-foreground">
                {sendOnEnter
                  ? "按 Enter 发送，Shift+Enter 换行"
                  : "按 Ctrl/Cmd+Enter 发送，Enter 换行"}
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
              <Label htmlFor="include-metadata">导出时包含消息元数据</Label>
              <p className="text-sm text-muted-foreground">
                导出消息时包含时间戳和消息编号
              </p>
            </div>
            <Switch
              id="include-metadata"
              checked={includeMetadataInExport}
              onCheckedChange={setIncludeMetadataInExport}
            />
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="use-heti">使用 heti 排版</Label>
              <p className="text-sm text-muted-foreground">
                使用 heti 排版，使中文排版内容更美观
              </p>
            </div>
            <Switch
              id="use-heti"
              checked={useHeti}
              onCheckedChange={setUseHeti}
            />
          </div>

          {/* Editor Font Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="editor-font-size">编辑器字体大小</Label>
            <div className="flex items-center gap-4">
              <Slider
                id="editor-font-size"
                min={12}
                max={24}
                step={1}
                value={[editorFontSize]}
                onValueChange={([value]) => setEditorFontSize(value)}
                className="flex-1"
              />
              <Input
                type="number"
                min={12}
                max={24}
                value={editorFontSize}
                onChange={(e) => {
                  const value = parseInt(e.target.value, 10);
                  if (!isNaN(value) && value >= 12 && value <= 24) {
                    setEditorFontSize(value);
                  }
                }}
                className="w-20"
              />
            </div>
            <p className="text-sm text-muted-foreground">
              调整 Markdown 编辑器的字体大小（12-24px）
            </p>
          </div>

          {/* Toolbar Button Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="toolbar-button-size">工具栏按钮大小</Label>
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
              调整编辑器工具栏按钮的大小（20-36px）
            </p>
          </div>

          {/* Message Font Size Setting */}
          <div className="space-y-3">
            <Label htmlFor="message-font-size">消息字体大小</Label>
            <div className="flex items-center gap-4">
              <Slider
                id="message-font-size"
                min={12}
                max={20}
                step={1}
                value={[messageFontSize]}
                onValueChange={([value]) => setMessageFontSize(value)}
                className="flex-1"
              />
              <Input
                type="number"
                min={12}
                max={20}
                value={messageFontSize}
                onChange={(e) => {
                  const value = parseInt(e.target.value, 10);
                  if (!isNaN(value) && value >= 12 && value <= 20) {
                    setMessageFontSize(value);
                  }
                }}
                className="w-20"
              />
            </div>
            <p className="text-sm text-muted-foreground">
              调整聊天消息的字体大小（12-20px）
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
