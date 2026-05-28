import type { Metadata } from "next";

import HomePage from "@/app/_components/home-page";

export const metadata: Metadata = {
  title: "Elizabeth - Secure File Sharing",
  description: "A modern, room-centric file sharing and collaboration platform",
  generator: "unic",
};



export default function Page() {
  return <HomePage />;
}
