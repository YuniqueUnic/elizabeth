"use client"

// Custom hook for theme management
import { useEffect } from "react"
import { useAppStore } from "../store"

export function useTheme() {
  const { theme, setTheme, cycleTheme } = useAppStore()

  useEffect(() => {
    const root = window.document.documentElement
    root.classList.remove("light", "dark")

    if (theme === "system") {
      const systemTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
      root.classList.add(systemTheme)
    } else {
      root.classList.add(theme)
    }
  }, [theme])

  return { theme, setTheme, cycleTheme }
}
