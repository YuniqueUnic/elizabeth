// Core type definitions for Elizabeth platform

export interface RoomSettings {
  expiresAt: string
  passwordProtected: boolean
  password?: string
  maxViews: number
}

export interface RoomDetails {
  id: string
  currentSize: number // in MB
  maxSize: number // in MB
  settings: RoomSettings
  permissions: RoomPermission[]
}

export type RoomPermission = "read" | "edit" | "share" | "delete"

export interface Message {
  id: string
  content: string
  timestamp: string
  user: string
}

export interface FileItem {
  id: string
  name: string
  thumbnailUrl: string | null
  size?: number // in bytes
  type?: "image" | "video" | "pdf" | "link" | "document"
  url?: string
}

export type Theme = "dark" | "light" | "system"
