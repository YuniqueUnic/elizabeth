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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Link as LinkIcon } from "lucide-react";

// 支持的协议列表
const URL_PROTOCOLS = [
  { value: "https://", label: "https://" },
  { value: "http://", label: "http://" },
  { value: "ftp://", label: "ftp://" },
  { value: "ftps://", label: "ftps://" },
  { value: "sftp://", label: "sftp://" },
  { value: "mailto:", label: "mailto:" },
  { value: "tel:", label: "tel:" },
  { value: "manual", label: "手动输入" },
] as const;

type ProtocolValue = (typeof URL_PROTOCOLS)[number]["value"];

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
  const [protocol, setProtocol] = useState<ProtocolValue>("https://");
  const [urlInput, setUrlInput] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [urlError, setUrlError] = useState("");

  // 根据协议和输入值构建完整 URL
  const buildFullUrl = (): string => {
    const input = urlInput.trim();
    if (protocol === "manual") {
      return input;
    }
    return `${protocol}${input}`;
  };

  const validateUrl = (fullUrl: string): boolean => {
    try {
      new URL(fullUrl);
      setUrlError("");
      return true;
    } catch {
      setUrlError("请输入有效的 URL");
      return false;
    }
  };

  // 验证当前输入
  const validateCurrentInput = (): boolean => {
    const fullUrl = buildFullUrl();
    return validateUrl(fullUrl);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!urlInput.trim()) {
      setUrlError("URL 不能为空");
      return;
    }

    const fullUrl = buildFullUrl();

    if (!validateUrl(fullUrl)) {
      return;
    }

    if (!name.trim()) {
      // Auto-generate name from URL if not provided
      try {
        const urlObj = new URL(fullUrl);
        const autoName = urlObj.hostname + urlObj.pathname;
        onSubmit({
          url: fullUrl,
          name: autoName || "未命名链接",
          description: description.trim() || undefined,
        });
      } catch {
        onSubmit({
          url: fullUrl,
          name: "未命名链接",
          description: description.trim() || undefined,
        });
      }
    } else {
      onSubmit({
        url: fullUrl,
        name: name.trim(),
        description: description.trim() || undefined,
      });
    }

    // Reset form
    setProtocol("https://");
    setUrlInput("");
    setName("");
    setDescription("");
    setUrlError("");
  };

  const handleCancel = () => {
    setProtocol("https://");
    setUrlInput("");
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
              <div className="flex gap-0">
                <Select
                  value={protocol}
                  onValueChange={(value: ProtocolValue) => {
                    setProtocol(value);
                    if (urlError) {
                      // 当协议改变时重新验证
                      const newFullUrl =
                        value === "manual"
                          ? urlInput.trim()
                          : `${value}${urlInput.trim()}`;
                      if (urlInput.trim()) validateUrl(newFullUrl);
                    }
                  }}
                  disabled={isUploading}
                >
                  <SelectTrigger className="w-[130px] rounded-r-none border-r-0 shrink-0">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {URL_PROTOCOLS.map((proto) => (
                      <SelectItem key={proto.value} value={proto.value}>
                        {proto.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  id="url"
                  type="text"
                  placeholder={
                    protocol === "manual"
                      ? "https://example.com/path"
                      : "example.com/path"
                  }
                  value={urlInput}
                  onChange={(e) => {
                    setUrlInput(e.target.value);
                    if (urlError) {
                      const newFullUrl =
                        protocol === "manual"
                          ? e.target.value.trim()
                          : `${protocol}${e.target.value.trim()}`;
                      if (e.target.value.trim()) validateUrl(newFullUrl);
                    }
                  }}
                  onBlur={() => urlInput && validateCurrentInput()}
                  disabled={isUploading}
                  className={`flex-1 rounded-l-none ${urlError ? "border-destructive" : ""}`}
                />
              </div>
              {urlError && (
                <p className="text-sm text-destructive">{urlError}</p>
              )}
              {protocol !== "manual" && urlInput.trim() && (
                <p className="text-xs text-muted-foreground">
                  完整链接：{buildFullUrl()}
                </p>
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
            <Button type="submit" disabled={isUploading || !urlInput.trim()}>
              {isUploading ? "添加中..." : "添加链接"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
