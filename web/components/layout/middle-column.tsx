"use client"

import { MessageList } from "@/components/chat/message-list"
import { MessageInput } from "@/components/chat/message-input"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { getMessages, postMessage, updateMessage, deleteMessage } from "@/api/roomService"
import { useAppStore } from "@/lib/store"
import { useState } from "react"
import type { Message } from "@/lib/types"
import { useToast } from "@/hooks/use-toast"

export function MiddleColumn() {
  const currentRoomId = useAppStore((state) => state.currentRoomId)
  const queryClient = useQueryClient()
  const [editingMessage, setEditingMessage] = useState<Message | null>(null)
  const { toast } = useToast()

  const { data: messages = [], isLoading } = useQuery({
    queryKey: ["messages", currentRoomId],
    queryFn: () => getMessages(currentRoomId),
  })

  const postMutation = useMutation({
    mutationFn: (content: string) => postMessage(currentRoomId, content),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] })
      toast({
        title: "消息已发送",
        description: "您的消息已成功发送",
      })
    },
    onError: () => {
      toast({
        title: "发送失败",
        description: "无法发送消息，请重试",
        variant: "destructive",
      })
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ messageId, content }: { messageId: string; content: string }) => updateMessage(messageId, content),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] })
      setEditingMessage(null)
      toast({
        title: "消息已更新",
        description: "您的消息已成功更新",
      })
    },
    onError: () => {
      toast({
        title: "更新失败",
        description: "无法更新消息，请重试",
        variant: "destructive",
      })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (messageId: string) => deleteMessage(messageId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["messages", currentRoomId] })
    },
  })

  const handleSend = (content: string) => {
    if (editingMessage) {
      updateMutation.mutate({ messageId: editingMessage.id, content })
    } else {
      postMutation.mutate(content)
    }
  }

  const handleEdit = (message: Message) => {
    setEditingMessage(message)
  }

  const handleCancelEdit = () => {
    setEditingMessage(null)
  }

  const handleDelete = (messageId: string) => {
    deleteMutation.mutate(messageId)
  }

  return (
    <main className="flex flex-1 flex-col overflow-hidden">
      <MessageList messages={messages} isLoading={isLoading} onEdit={handleEdit} onDelete={handleDelete} />
      <MessageInput
        onSend={handleSend}
        editingMessage={editingMessage}
        onCancelEdit={handleCancelEdit}
        isLoading={postMutation.isPending || updateMutation.isPending}
      />
    </main>
  )
}
