"use client"

import { Button } from "@/components/ui/button"
import { Bold, Italic, Code, List, ListOrdered, Link2, Heading } from "lucide-react"

interface MarkdownToolbarProps {
  onInsert: (before: string, after?: string) => void
}

export function MarkdownToolbar({ onInsert }: MarkdownToolbarProps) {
  const tools = [
    { icon: Bold, label: "粗体", before: "**", after: "**" },
    { icon: Italic, label: "斜体", before: "*", after: "*" },
    { icon: Code, label: "代码", before: "`", after: "`" },
    { icon: Heading, label: "标题", before: "# ", after: "" },
    { icon: List, label: "无序列表", before: "- ", after: "" },
    { icon: ListOrdered, label: "有序列表", before: "1. ", after: "" },
    { icon: Link2, label: "链接", before: "[", after: "](url)" },
  ]

  return (
    <div className="mb-2 flex flex-wrap gap-1">
      {tools.map((tool) => (
        <Button
          key={tool.label}
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => onInsert(tool.before, tool.after)}
          title={tool.label}
        >
          <tool.icon className="h-4 w-4" />
        </Button>
      ))}
    </div>
  )
}
