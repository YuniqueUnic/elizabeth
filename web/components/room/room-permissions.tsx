"use client"

import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import type { RoomPermission } from "@/lib/types"

interface RoomPermissionsProps {
  permissions: RoomPermission[]
}

const permissionLabels: Record<RoomPermission, string> = {
  read: "只读",
  edit: "编辑",
  share: "分享",
  delete: "删除",
}

export function RoomPermissions({ permissions }: RoomPermissionsProps) {
  const allPermissions: RoomPermission[] = ["read", "edit", "share", "delete"]

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold">房间权限</h3>
      <div className="space-y-2">
        {allPermissions.map((permission) => (
          <div key={permission} className="flex items-center space-x-2">
            <Checkbox id={permission} checked={permissions.includes(permission)} disabled />
            <Label htmlFor={permission} className="text-sm font-normal cursor-pointer">
              {permissionLabels[permission]}
            </Label>
          </div>
        ))}
      </div>
    </div>
  )
}
