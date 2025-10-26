"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { FolderOpen, MessageSquare, Settings } from "lucide-react";
import { LeftSidebar } from "./left-sidebar";
import { MiddleColumn } from "./middle-column";
import { RightSidebar } from "./right-sidebar";

export function MobileLayout() {
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

                    <TabsContent value="chat" className="h-full m-0">
                        <MiddleColumn />
                    </TabsContent>

                    <TabsContent value="files" className="h-full m-0">
                        <div className="h-full overflow-hidden">
                            <RightSidebar />
                        </div>
                    </TabsContent>
                </div>

                {/* Bottom Tab Bar */}
                <TabsList className="w-full rounded-none h-14 grid grid-cols-3 border-t">
                    <TabsTrigger
                        value="settings"
                        className="flex-col gap-1 h-full data-[state=active]:bg-accent"
                    >
                        <Settings className="h-5 w-5" />
                        <span className="text-xs">设置</span>
                    </TabsTrigger>
                    <TabsTrigger
                        value="chat"
                        className="flex-col gap-1 h-full data-[state=active]:bg-accent"
                    >
                        <MessageSquare className="h-5 w-5" />
                        <span className="text-xs">聊天</span>
                    </TabsTrigger>
                    <TabsTrigger
                        value="files"
                        className="flex-col gap-1 h-full data-[state=active]:bg-accent"
                    >
                        <FolderOpen className="h-5 w-5" />
                        <span className="text-xs">文件</span>
                    </TabsTrigger>
                </TabsList>
            </Tabs>
        </div>
    );
}
