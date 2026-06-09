"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import React from "react";

interface ToolbarButtonProps {
  onClick: () => void;
  active?: boolean;
  children: React.ReactNode;
  title: string;
  disabled?: boolean;
}

export const ToolbarButton = ({
  onClick,
  active,
  children,
  title,
  disabled
}: ToolbarButtonProps) => {
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "h-7 w-7 p-0 rounded-md transition-all duration-200 text-muted-foreground hover:bg-muted/65 hover:text-foreground",
        active && "bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary"
      )}
      title={title}
      aria-label={title}
    >
      {children}
    </Button>
  );
};
