"use client";

import { useTranslations } from "next-intl";

import { CopyButton } from "@/components/copy-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

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
        <CopyButton value={code} label={t("copyIdentityCode")} />
      </div>
      <p className="text-xs text-muted-foreground">{t("createdIdentityCodeHint")}</p>
      <Button type="button" className="w-full" onClick={onEnter} data-testid="enter-room">
        {t("enterRoom")}
      </Button>
    </div>
  );
}
