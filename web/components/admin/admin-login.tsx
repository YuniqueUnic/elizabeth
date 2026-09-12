"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/** 登录页（展示组件）：token 仅进入服务端校验，前端不做授权判定。 */
export function AdminLogin({
  onSubmit,
}: {
  onSubmit: (adminToken: string) => void;
}) {
  const t = useTranslations("admin");
  const [token, setToken] = useState("");

  return (
    <div className="flex min-h-[60vh] items-center justify-center">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>{t("login.title")}</CardTitle>
          <CardDescription>{t("login.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
              const trimmed = token.trim();
              if (trimmed) onSubmit(trimmed);
            }}
          >
            <div className="space-y-2">
              <Label htmlFor="admin-token">{t("login.tokenLabel")}</Label>
              <Input
                id="admin-token"
                type="password"
                autoComplete="off"
                placeholder={t("login.tokenPlaceholder")}
                value={token}
                onChange={(event) => setToken(event.target.value)}
              />
            </div>
            <Button type="submit" className="w-full">
              {t("login.submit")}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
