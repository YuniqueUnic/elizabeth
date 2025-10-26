"use client"

import type React from "react"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { useAppStore } from "@/lib/store"

export function SettingsDialog({ children }: { children: React.ReactNode }) {
  const { sendOnEnter, setSendOnEnter, includeMetadataInExport, setIncludeMetadataInExport } = useAppStore()

  return (
    <Dialog>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>设置</DialogTitle>
          <DialogDescription>配置应用程序的行为和偏好设置</DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Send on Enter Setting */}
          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="send-on-enter">按 Enter 键发送消息</Label>
              <p className="text-sm text-muted-foreground">
                {sendOnEnter ? "按 Enter 发送，Shift+Enter 换行" : "按 Ctrl/Cmd+Enter 发送，Enter 换行"}
              </p>
            </div>
            <Switch id="send-on-enter" checked={sendOnEnter} onCheckedChange={setSendOnEnter} />
          </div>

          <div className="flex items-center justify-between space-x-4">
            <div className="flex-1 space-y-1">
              <Label htmlFor="include-metadata">导出时包含消息元数据</Label>
              <p className="text-sm text-muted-foreground">导出消息时包含时间戳和消息编号</p>
            </div>
            <Switch
              id="include-metadata"
              checked={includeMetadataInExport}
              onCheckedChange={setIncludeMetadataInExport}
            />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
