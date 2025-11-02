"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Link as LinkIcon } from "lucide-react";

interface UrlUploadDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (data: UrlUploadData) => void;
  isUploading: boolean;
}

export interface UrlUploadData {
  url: string;
  name: string;
  description?: string;
}

export function UrlUploadDialog({
  open,
  onOpenChange,
  onSubmit,
  isUploading,
}: UrlUploadDialogProps) {
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [urlError, setUrlError] = useState("");

  const validateUrl = (value: string): boolean => {
    try {
      new URL(value);
      setUrlError("");
      return true;
    } catch {
      setUrlError("请输入有效的 URL");
      return false;
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!url.trim()) {
      setUrlError("URL 不能为空");
      return;
    }

    if (!validateUrl(url.trim())) {
      return;
    }

    if (!name.trim()) {
      // Auto-generate name from URL if not provided
      try {
        const urlObj = new URL(url.trim());
        const autoName = urlObj.hostname + urlObj.pathname;
        onSubmit({
          url: url.trim(),
          name: autoName || "未命名链接",
          description: description.trim() || undefined,
        });
      } catch {
        onSubmit({
          url: url.trim(),
          name: "未命名链接",
          description: description.trim() || undefined,
        });
      }
    } else {
      onSubmit({
        url: url.trim(),
        name: name.trim(),
        description: description.trim() || undefined,
      });
    }

    // Reset form
    setUrl("");
    setName("");
    setDescription("");
    setUrlError("");
  };

  const handleCancel = () => {
    setUrl("");
    setName("");
    setDescription("");
    setUrlError("");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <LinkIcon className="h-5 w-5" />
            添加链接
          </DialogTitle>
          <DialogDescription>
            添加一个外部链接到房间。链接将被保存并可以在文件列表中查看。
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="url">
                URL <span className="text-destructive">*</span>
              </Label>
              <Input
                id="url"
                type="url"
                placeholder="https://example.com"
                value={url}
                onChange={(e) => {
                  setUrl(e.target.value);
                  if (urlError) validateUrl(e.target.value);
                }}
                onBlur={() => url && validateUrl(url)}
                disabled={isUploading}
                className={urlError ? "border-destructive" : ""}
              />
              {urlError && (
                <p className="text-sm text-destructive">{urlError}</p>
              )}
            </div>

            <div className="grid gap-2">
              <Label htmlFor="name">
                显示名称
                <span className="text-muted-foreground text-xs ml-2">
                  (可选，留空将自动生成)
                </span>
              </Label>
              <Input
                id="name"
                type="text"
                placeholder="我的链接"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={isUploading}
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="description">
                描述
                <span className="text-muted-foreground text-xs ml-2">
                  (可选)
                </span>
              </Label>
              <Textarea
                id="description"
                placeholder="链接的简短描述..."
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                disabled={isUploading}
                rows={3}
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              disabled={isUploading}
            >
              取消
            </Button>
            <Button type="submit" disabled={isUploading || !url.trim()}>
              {isUploading ? "添加中..." : "添加链接"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
