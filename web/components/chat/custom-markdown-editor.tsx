"use client"

import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { MarkdownRenderer } from "./markdown-renderer"
import { Bold, Italic, Code, List, ListOrdered, Link2, Heading, ImageIcon, Quote, Minus } from "lucide-react"
import { useRef, type KeyboardEvent } from "react"

interface CustomMarkdownEditorProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  height?: number
  showPreview?: boolean
}

export function CustomMarkdownEditor({
  value,
  onChange,
  placeholder,
  height = 120,
  showPreview = false,
}: CustomMarkdownEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const insertMarkdown = (before: string, after = "") => {
    const textarea = textareaRef.current
    if (!textarea) return

    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const selectedText = value.substring(start, end)
    const beforeText = value.substring(0, start)
    const afterText = value.substring(end)

    const newText = beforeText + before + selectedText + after + afterText
    onChange(newText)

    // Set cursor position after insertion
    setTimeout(() => {
      textarea.focus()
      const newCursorPos = start + before.length + selectedText.length
      textarea.setSelectionRange(newCursorPos, newCursorPos)
    }, 0)
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Tab key for indentation
    if (e.key === "Tab") {
      e.preventDefault()
      insertMarkdown("  ")
    }
  }

  const tools = [
    { icon: Bold, label: "粗体 (Ctrl+B)", before: "**", after: "**", shortcut: "b" },
    { icon: Italic, label: "斜体 (Ctrl+I)", before: "*", after: "*", shortcut: "i" },
    { icon: Code, label: "行内代码", before: "`", after: "`" },
    { icon: Heading, label: "标题", before: "## ", after: "" },
    { icon: List, label: "无序列表", before: "- ", after: "" },
    { icon: ListOrdered, label: "有序列表", before: "1. ", after: "" },
    { icon: Link2, label: "链接", before: "[", after: "](url)" },
    { icon: ImageIcon, label: "图片", before: "![alt](", after: ")" },
    { icon: Quote, label: "引用", before: "> ", after: "" },
    { icon: Minus, label: "分隔线", before: "\n---\n", after: "" },
  ]

  if (!showPreview) {
    return (
      <div className="space-y-2">
        {/* Toolbar */}
        <div className="flex flex-wrap gap-1 border-b pb-2">
          {tools.map((tool) => (
            <Button
              key={tool.label}
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => insertMarkdown(tool.before, tool.after)}
              title={tool.label}
              type="button"
            >
              <tool.icon className="h-4 w-4" />
            </Button>
          ))}
        </div>

        {/* Editor */}
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          className="w-full resize-none rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring font-mono"
          style={{ height: `${height}px` }}
        />
      </div>
    )
  }

  return (
    <Tabs defaultValue="edit" className="w-full h-full flex flex-col">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="edit">编辑</TabsTrigger>
        <TabsTrigger value="preview">预览</TabsTrigger>
        <TabsTrigger value="split">分屏</TabsTrigger>
      </TabsList>

      <TabsContent value="edit" className="flex-1 mt-4 space-y-2">
        {/* Toolbar */}
        <div className="flex flex-wrap gap-1 border-b pb-2">
          {tools.map((tool) => (
            <Button
              key={tool.label}
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => insertMarkdown(tool.before, tool.after)}
              title={tool.label}
              type="button"
            >
              <tool.icon className="h-4 w-4" />
            </Button>
          ))}
        </div>

        {/* Editor */}
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          className="w-full h-[calc(100%-60px)] resize-none rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring font-mono"
        />
      </TabsContent>

      <TabsContent value="preview" className="flex-1 mt-4">
        <div className="h-full overflow-auto rounded-md border bg-background p-4">
          <MarkdownRenderer content={value || "*预览区域为空*"} />
        </div>
      </TabsContent>

      <TabsContent value="split" className="flex-1 mt-4">
        <div className="grid grid-cols-2 gap-4 h-full">
          {/* Editor Side */}
          <div className="space-y-2">
            <div className="flex flex-wrap gap-1 border-b pb-2">
              {tools.map((tool) => (
                <Button
                  key={tool.label}
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8"
                  onClick={() => insertMarkdown(tool.before, tool.after)}
                  title={tool.label}
                  type="button"
                >
                  <tool.icon className="h-4 w-4" />
                </Button>
              ))}
            </div>
            <textarea
              ref={textareaRef}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={placeholder}
              className="w-full h-[calc(100%-60px)] resize-none rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring font-mono"
            />
          </div>

          {/* Preview Side */}
          <div className="h-full overflow-auto rounded-md border bg-background p-4">
            <MarkdownRenderer content={value || "*预览区域为空*"} />
          </div>
        </div>
      </TabsContent>
    </Tabs>
  )
}
