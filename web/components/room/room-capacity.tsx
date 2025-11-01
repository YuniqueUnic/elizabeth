"use client";

import { Progress } from "@/components/ui/progress";

interface RoomCapacityProps {
  currentSize: number; // in bytes
  maxSize: number; // in bytes
}

export function RoomCapacity({ currentSize, maxSize }: RoomCapacityProps) {
  // Convert bytes to MB
  const currentSizeMB = currentSize / (1024 * 1024);
  const maxSizeMB = maxSize / (1024 * 1024);
  const percentage = (currentSize / maxSize) * 100;

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold">容量使用</h3>
      <div className="space-y-2">
        <Progress value={percentage} className="h-2" />
        <p className="text-sm text-muted-foreground">
          {currentSizeMB.toFixed(1)} MB / {maxSizeMB.toFixed(1)}{" "}
          MB ({percentage.toFixed(1)}%)
        </p>
      </div>
    </div>
  );
}
