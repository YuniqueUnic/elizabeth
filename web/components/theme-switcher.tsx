"use client";

import { Button } from "@/components/ui/button";
import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "@/lib/hooks/use-theme";

export function ThemeSwitcher() {
  const { theme, cycleTheme } = useTheme();

  const getIcon = () => {
    switch (theme) {
      case "dark":
        return <Moon className="h-4 w-4" />;
      case "light":
        return <Sun className="h-4 w-4" />;
      case "system":
        return <Monitor className="h-4 w-4" />;
    }
  };

  const getTitle = () => {
    switch (theme) {
      case "dark":
        return "暗色主题";
      case "light":
        return "亮色主题";
      case "system":
        return "跟随系统";
    }
  };

  return (
    <Button variant="ghost" size="icon" onClick={cycleTheme} title={getTitle()}>
      {getIcon()}
    </Button>
  );
}
