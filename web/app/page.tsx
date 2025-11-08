import type { Metadata } from "next";

import HomePage from "@/app/_components/home-page";

export const metadata: Metadata = {
  title: "Elizabeth - Secure File Sharing",
  description: "A modern, room-centric file sharing and collaboration platform",
  generator: "v0.app",
};

export const dynamic = "force-dynamic";
export const revalidate = 0;

export default function Page() {
  return <HomePage />;
}
