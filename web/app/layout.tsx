import type React from "react";
import type { Metadata } from "next";

import "./globals.css";
import "./shiki.css";
import { Providers } from "@/components/providers";
import { Toaster } from "@/components/ui/toaster";
import { FontSizeManager } from "@/components/font-size-manager";

export const metadata: Metadata = {
  title: "Elizabeth - Secure File Sharing",
  description: "A modern, room-centric file sharing and collaboration platform",
  generator: "v0.app",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <head>
        <link rel="stylesheet" href="https://unpkg.com/heti/umd/heti.min.css" />
      </head>
      <body className="font-sans antialiased">
        <Providers>
          <FontSizeManager />
          {children}
          <Toaster />
        </Providers>
      </body>
    </html>
  );
}
