/**
 * Room Events Hook for Elizabeth Platform
 *
 * Provides real-time room event handling with automatic cache invalidation
 * and data refresh through TanStack Query integration.
 *
 * Features:
 * - Subscribe to room WebSocket events
 * - Handle CONTENT_CREATED, CONTENT_UPDATED, CONTENT_DELETED events
 * - Automatic TanStack Query cache invalidation
 */

import { useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { useWebSocket, WsMessageType, type WsMessage } from "./use-websocket";
import type { ContentEventPayload, RoomUpdatePayload } from "./use-websocket";
import { ContentType, parseContentType } from "../types";

// ============================================================================
// Query Key Factories
// ============================================================================

/**
 * TanStack Query key factories for cache invalidation
 */
export const queryKeys = {
  /**
   * Query key for room files list
   */
  roomFiles: (roomName: string) => ["files", roomName] as const,

  /**
   * Query key for room messages list
   */
  roomMessages: (roomName: string) => ["messages", roomName] as const,

  /**
   * Query key for room details
   */
  roomDetails: (roomName: string) => ["room", roomName] as const,

  /**
   * Query key for room settings
   */
  roomSettings: (roomName: string) => ["room", roomName] as const,
} as const;

// ============================================================================
// Hook Options
// ============================================================================

export interface UseRoomEventsOptions {
  /** WebSocket server URL */
  wsUrl: string;
  /** Room name */
  roomName: string;
  /** Room token for authentication */
  token: string;
  /** Callback when content is created */
  onContentCreated?: (payload: ContentEventPayload) => void;
  /** Callback when content is updated */
  onContentUpdated?: (payload: ContentEventPayload) => void;
  /** Callback when content is deleted */
  onContentDeleted?: (payload: ContentEventPayload) => void;
  /** Callback when room is updated */
  onRoomUpdate?: (payload: RoomUpdatePayload) => void;
  /** Callback after the websocket reconnects successfully */
  onReconnected?: () => void;
  /**
   * 服务端下发终止性错误帧（握手拒绝 / 订阅失败 / 房间删除或过期 / 会话被踢）。
   * 这类失败重连必然复现，消费方应回到进入流程而不是依赖重连。
   */
  onSessionInvalidated?: (reason: string) => void;
  /** Enable automatic cache invalidation */
  enableCacheInvalidation?: boolean;
}

// ============================================================================
// Main Hook
// ============================================================================

/**
 * Room events hook for real-time updates
 *
 * This hook:
 * 1. Connects to the room's WebSocket
 * 2. Listens for room events
 * 3. Invalidates relevant TanStack Query caches
 * 4. Calls optional event callbacks
 *
 * @param options - Room events configuration
 */
export function useRoomEvents(options: UseRoomEventsOptions) {
  const {
    wsUrl,
    roomName,
    token,
    onContentCreated,
    onContentUpdated,
    onContentDeleted,
    onRoomUpdate,
    onReconnected,
    onSessionInvalidated,
    enableCacheInvalidation = true,
  } = options;

  const queryClient = useQueryClient();

  /**
   * Invalidate relevant queries when content changes
   */
  const invalidateContentQueries = useCallback((payload: ContentEventPayload) => {
    if (!enableCacheInvalidation) return;

    if (parseContentType(payload.content_type) !== ContentType.Text) {
      queryClient.invalidateQueries({
        queryKey: queryKeys.roomFiles(roomName),
      });
    }
  }, [roomName, queryClient, enableCacheInvalidation]);

  /**
   * Invalidate room details/settings queries
   */
  const invalidateRoomQueries = useCallback(() => {
    if (!enableCacheInvalidation) return;

    queryClient.invalidateQueries({
      queryKey: queryKeys.roomDetails(roomName),
    });
    queryClient.invalidateQueries({
      queryKey: queryKeys.roomSettings(roomName),
    });
  }, [roomName, queryClient, enableCacheInvalidation]);

  /**
   * Handle incoming WebSocket messages
   */
  const handleMessage = useCallback((message: WsMessage) => {
    switch (message.message_type) {
      case WsMessageType.ContentCreated: {
        const payload = message.payload as ContentEventPayload;
        onContentCreated?.(payload);
        invalidateContentQueries(payload);
        break;
      }

      case WsMessageType.ContentUpdated: {
        const payload = message.payload as ContentEventPayload;
        onContentUpdated?.(payload);
        invalidateContentQueries(payload);
        break;
      }

      case WsMessageType.ContentDeleted: {
        const payload = message.payload as ContentEventPayload;
        onContentDeleted?.(payload);
        invalidateContentQueries(payload);
        break;
      }

      case WsMessageType.RoomUpdate: {
        const payload = message.payload as RoomUpdatePayload;
        onRoomUpdate?.(payload);
        invalidateRoomQueries();
        break;
      }

      default:
        // PING / ERROR 已由 use-websocket 在传输层消费；这里忽略其余控制帧（CONNECT_ACK、PONG）。
        break;
    }
  }, [
    onContentCreated,
    onContentUpdated,
    onContentDeleted,
    onRoomUpdate,
    invalidateContentQueries,
    invalidateRoomQueries,
  ]);

  /**
   * WebSocket connection
   */
  const ws = useWebSocket({
    url: wsUrl,
    token,
    roomName,
    onMessage: handleMessage,
    onReconnected,
    onErrorFrame: onSessionInvalidated,
    enableReconnect: true,
  });

  /**
   * Effect: Log connection state changes (for debugging)
   */
  useEffect(() => {
    if (ws.connected) {
      console.debug(`[useRoomEvents] Connected to room: ${roomName}`);
    }
    if (ws.error) {
      console.error(`[useRoomEvents] WebSocket error for room ${roomName}:`, ws.error);
    }
  }, [ws.connected, ws.error, roomName]);

  return {
    ...ws,
    /**
     * Manually trigger a refresh of all room-related queries
     */
    refreshQueries: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.roomFiles(roomName),
      });
      invalidateRoomQueries();
    },
  };
}

// ============================================================================
// Re-exports
// ============================================================================

export type { ContentEventPayload, RoomUpdatePayload };
