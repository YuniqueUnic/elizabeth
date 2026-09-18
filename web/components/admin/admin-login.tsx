"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/** 登录页（展示组件）：凭证只进入服务端校验，前端不做授权判定。 */
export function AdminLogin({
  onSubmit,
}: {
  onSubmit: (username: string, password: string) => void;
}) {
  const t = useTranslations("admin");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const ready = username.trim() !== "" && password !== "";

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
              if (ready) onSubmit(username.trim(), password);
            }}
          >
            <div className="space-y-2">
              <Label htmlFor="admin-username">{t("login.usernameLabel")}</Label>
              <Input
                id="admin-username"
                autoComplete="username"
                placeholder={t("login.usernamePlaceholder")}
                value={username}
                onChange={(event) => setUsername(event.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="admin-password">{t("login.passwordLabel")}</Label>
              <Input
                id="admin-password"
                type="password"
                autoComplete="current-password"
                placeholder={t("login.passwordPlaceholder")}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </div>
            <Button type="submit" className="w-full" disabled={!ready}>
              {t("login.submit")}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
