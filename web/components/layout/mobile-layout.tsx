"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { FolderOpen, MessageSquare, Settings } from "lucide-react";
import { useTranslations } from "next-intl";
import { LeftSidebar } from "./left-sidebar";
import { MiddleColumn } from "./middle-column";
import { RightSidebar } from "./right-sidebar";

export function MobileLayout() {
  const t = useTranslations("common");
  return (
    <div className="flex h-full flex-col overflow-hidden">
      <Tabs defaultValue="chat" className="flex h-full flex-col">
        {/* Tab Content Area */}
        <div className="flex-1 overflow-hidden">
          <TabsContent value="settings" className="h-full m-0">
            <div className="h-full overflow-auto">
              <LeftSidebar />
            </div>
          </TabsContent>

          <TabsContent
            value="chat"
            className="h-full m-0 data-[state=active]:flex data-[state=active]:flex-col"
          >
            <MiddleColumn />
          </TabsContent>

          <TabsContent value="files" className="h-full m-0">
            <div className="h-full overflow-hidden">
              <RightSidebar />
            </div>
          </TabsContent>
        </div>

        {/* Bottom Tab Bar */}
        <TabsList
          className="grid h-11 w-full grid-cols-3 rounded-none border-t"
          data-testid="mobile-bottom-tabs"
        >
          <TabsTrigger
            value="settings"
            className="h-full flex-col gap-0.5 data-[state=active]:bg-accent"
          >
            <Settings className="h-4 w-4" />
            <span className="text-[11px] leading-none">{t("mobileTabSettings")}</span>
          </TabsTrigger>
          <TabsTrigger
            value="chat"
            className="h-full flex-col gap-0.5 data-[state=active]:bg-accent"
          >
            <MessageSquare className="h-4 w-4" />
            <span className="text-[11px] leading-none">{t("mobileTabChat")}</span>
          </TabsTrigger>
          <TabsTrigger
            value="files"
            className="h-full flex-col gap-0.5 data-[state=active]:bg-accent"
          >
            <FolderOpen className="h-4 w-4" />
            <span className="text-[11px] leading-none">{t("mobileTabFiles")}</span>
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </div>
  );
}
