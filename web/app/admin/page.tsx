"use client";

import { useTranslations } from "next-intl";

import { AdminPanel } from "@/components/admin/admin-panel";

export default function AdminPage() {
  const t = useTranslations("admin");

  return (
    <main className="mx-auto w-full max-w-6xl space-y-6 px-4 py-8">
      <div>
        <h1 className="text-2xl font-semibold">{t("title")}</h1>
        <p className="text-muted-foreground mt-1 text-sm">{t("description")}</p>
      </div>
      <AdminPanel />
    </main>
  );
}
