"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { ManualCopyDialog } from "@/components/manual-copy-dialog";

/**
 * 一次性身份码披露面板：创建者唯一一次看到 admin 身份码的地方。
 * 首页创建与 URL 直达开通两条入口共用，保证凭据披露语义一致。
 */
export function IdentityCodeDisclosure({
  code,
  onEnter,
}: {
  code: string;
  onEnter: () => void;
}) {
  const t = useTranslations("home");
  const [manualCopyValue, setManualCopyValue] = useState("");

  return (
    <div
      className="space-y-2 border border-primary/40 bg-primary/5 p-3"
      data-testid="identity-code-disclosure"
    >
      <Label>{t("createdIdentityCode")}</Label>
      <div className="flex gap-2">
        <Input
          readOnly
          value={code}
          onFocus={(event) => event.currentTarget.select()}
          className="font-mono text-xs"
          data-testid="disclosed-identity-code"
        />
        <Button
          type="button"
          size="icon"
          title={t("copyIdentityCode")}
          onClick={() =>
            void copyTextToClipboard(code).catch(() => setManualCopyValue(code))
          }
        >
          <Copy className="h-4 w-4" />
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">{t("createdIdentityCodeHint")}</p>
      <Button type="button" className="w-full" onClick={onEnter} data-testid="enter-room">
        {t("enterRoom")}
      </Button>
      <ManualCopyDialog
        open={manualCopyValue.length > 0}
        value={manualCopyValue}
        onOpenChange={(open) => !open && setManualCopyValue("")}
      />
    </div>
  );
}
