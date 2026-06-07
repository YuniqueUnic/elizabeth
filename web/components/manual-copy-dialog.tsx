"use client";

import { useEffect, useId, useRef } from "react";
import { useTranslations } from "next-intl";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

interface ManualCopyDialogProps {
  open: boolean;
  value: string;
  onOpenChange: (open: boolean) => void;
  title?: string;
  description?: string;
}

export function ManualCopyDialog(
  { open, value, onOpenChange, title, description }: ManualCopyDialogProps,
) {
  const t = useTranslations("common");
  const textareaId = useId();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!open) return;

    const timer = window.setTimeout(() => {
      textareaRef.current?.focus();
      textareaRef.current?.select();
    }, 50);

    return () => window.clearTimeout(timer);
  }, [open, value]);

  const selectValue = () => {
    textareaRef.current?.focus();
    textareaRef.current?.select();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{title ?? t("manualCopyTitle")}</DialogTitle>
          <DialogDescription>
            {description ?? t("manualCopyDescription")}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-2">
          <label className="sr-only" htmlFor={textareaId}>
            {t("manualCopyFieldLabel")}
          </label>
          <Textarea
            ref={textareaRef}
            id={textareaId}
            readOnly
            value={value}
            rows={value.length > 160 ? 6 : 3}
            className="resize-none font-mono text-xs leading-relaxed"
            data-testid="manual-copy-textarea"
            onFocus={(event) => event.currentTarget.select()}
            onClick={(event) => event.currentTarget.select()}
          />
          <p className="text-xs text-muted-foreground">
            {t("manualCopyShortcut")}
          </p>
        </div>

        <DialogFooter>
          <Button variant="outline" type="button" onClick={selectValue}>
            {t("manualCopySelectAll")}
          </Button>
          <Button type="button" onClick={() => onOpenChange(false)}>
            {t("manualCopyClose")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
