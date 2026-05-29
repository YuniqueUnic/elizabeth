"use client";

import { Button } from "@/components/ui/button";
import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "@/lib/hooks/use-theme";
import { useTranslations } from "next-intl";

export function ThemeSwitcher() {
  const { theme, cycleTheme } = useTheme();
  const t = useTranslations("common");

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
        return t("themeDark");
      case "light":
        return t("themeLight");
      case "system":
        return t("themeSystem");
    }
  };

  return (
    <Button variant="ghost" size="icon" onClick={cycleTheme} title={getTitle()}>
      {getIcon()}
    </Button>
  );
}
