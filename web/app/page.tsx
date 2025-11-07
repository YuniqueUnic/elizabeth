import type { Metadata } from "next";
import { redirect } from "next/navigation";

import HomePage from "@/app/_components/home-page";

export const metadata: Metadata = {
  title: "Elizabeth - Secure File Sharing",
  description: "A modern, room-centric file sharing and collaboration platform",
  generator: "v0.app",
};

export const dynamic = "force-dynamic";

export default function Page({
  searchParams,
}: {
  searchParams?: Record<string, string | string[] | undefined>;
}) {
  const ts = searchParams?._ts;

  if (!ts || Array.isArray(ts)) {
    const nextTs = Date.now();
    redirect(`/?_ts=${nextTs}`);
  }

  return <HomePage />;
}
