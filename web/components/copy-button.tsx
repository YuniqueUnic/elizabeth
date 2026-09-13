"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { Check, Copy } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { cn } from "@/lib/utils";

/**
 * 带成功反馈的复制按钮：图标切换 + 缩放动画 + aria-live 播报。
 * 浏览器阻止写入剪贴板时回退到手动复制弹窗。
 */
export function CopyButton({
  value,
  label,
  className,
  variant = "outline",
}: {
  value: string;
  /** 无障碍标签（默认「复制」） */
  label?: string;
  className?: string;
  variant?: "outline" | "ghost" | "default" | "secondary";
}) {
  const t = useTranslations("common");
  const [copied, setCopied] = useState(false);
  const [manualValue, setManualValue] = useState("");

  async function handleCopy() {
    try {
      await copyTextToClipboard(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setManualValue(value);
    }
  }

  return (
    <>
      <Button
        type="button"
        size="icon"
        variant={copied ? "default" : variant}
        className={cn("transition-all duration-200", className)}
        title={label ?? t("copy")}
        aria-label={label ?? t("copy")}
        onClick={() => void handleCopy()}
      >
        {copied ? (
          <Check className="h-4 w-4 animate-in zoom-in duration-200" />
        ) : (
          <Copy className="h-4 w-4" />
        )}
      </Button>
      <span role="status" aria-live="polite" className="sr-only">
        {copied ? t("copied") : ""}
      </span>
      <ManualCopyDialog
        open={manualValue.length > 0}
        value={manualValue}
        onOpenChange={(open) => !open && setManualValue("")}
      />
    </>
  );
}
