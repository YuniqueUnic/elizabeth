"use client";

import { Progress } from "@/components/ui/progress";
import { useTranslations } from "next-intl";

interface RoomCapacityProps {
  currentSize: number; // in bytes
  maxSize: number; // in bytes
}

export function RoomCapacity({ currentSize, maxSize }: RoomCapacityProps) {
  const t = useTranslations("room");
  // Convert bytes to MB
  const currentSizeMB = currentSize / (1024 * 1024);
  const maxSizeMB = maxSize / (1024 * 1024);
  const percentage = (currentSize / maxSize) * 100;

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold">{t("capacity.title")}</h3>
      <div className="space-y-2">
        <Progress value={percentage} className="h-2" />
        <p className="text-sm text-muted-foreground">
          {t("capacity.usage", {
            currentSize: currentSizeMB.toFixed(1),
            maxSize: maxSizeMB.toFixed(1),
            percentage: percentage.toFixed(1),
          })}
        </p>
      </div>
    </div>
  );
}
