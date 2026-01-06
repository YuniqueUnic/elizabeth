/**
 * WebSocket Hook for Elizabeth Platform
 *
 * Provides WebSocket connection management with automatic reconnection
 * for real-time room events.
 *
 * Features:
 * - Connection state management
 * - Exponential backoff reconnection
 * - Heartbeat (PING/PONG) handling
 * - Event message handling
 */

import { useCallback, useEffect, useRef, useState } from "react";

// ============================================================================
// Type Definitions (matching backend WebSocket protocol)
// ============================================================================

/**
 * WebSocket message types (matching backend WsMessageType enum)
 */
export enum WsMessageType {
  Connect = "connect",
  ConnectAck = "connect_ack",
  Ping = "ping",
  Pong = "pong",
  Error = "error",
  ContentCreated = "content_created",
  ContentUpdated = "content_updated",
  ContentDeleted = "content_deleted",
  UserJoined = "user_joined",
  UserLeft = "user_left",
  RoomUpdate = "room_update",
}

/**
 * WebSocket message structure (matching backend WsMessage)
 */
export interface WsMessage {
  message_type: WsMessageType;
  payload?: unknown;
  timestamp: number;
}

/**
 * Connection request payload
 */
export interface ConnectRequest {
  token: string;
  room_name: string;
}

/**
 * Connection acknowledgment response
 */
export interface ConnectAck {
  success: boolean;
  message: string;
  room_info?: RoomInfo;
}

/**
 * Room information from connection
 */
export interface RoomInfo {
  id: number;
  name: string;
  slug: string;
  max_size: number;
  current_size: number;
  max_times_entered: number;
  current_times_entered: number;
}

/**
 * WebSocket event payload types
 */
export interface ContentEventPayload {
  content_id: number;
  room_id: number;
}

export interface UserEventPayload {
  user_id: string;
  room_name: string;
}

// ============================================================================
// Configuration
// ============================================================================

const WS_CONFIG = {
  MAX_RECONNECT_ATTEMPTS: 5,
  BASE_RECONNECT_DELAY: 1000, // 1 second
  MAX_RECONNECT_DELAY: 30000, // 30 seconds
  HEARTBEAT_INTERVAL: 30000, // 30 seconds
} as const;

// ============================================================================
// Hook Options
// ============================================================================

export interface UseWebSocketOptions {
  /** WebSocket server URL (e.g., ws://localhost:8080/ws) */
  url: string;
  /** Room token for authentication */
  token: string;
  /** Room name */
  roomName: string;
  /** Callback when connection is established */
  onOpen?: (event: Event) => void;
  /** Callback when a message is received */
  onMessage?: (message: WsMessage) => void;
  /** Callback when connection is closed */
  onClose?: (event: CloseEvent) => void;
  /** Callback when an error occurs */
  onError?: (event: Event) => void;
  /** Callback when reconnection happens */
  onReconnect?: (attempt: number) => void;
  /** Enable automatic reconnection */
  enableReconnect?: boolean;
}

// ============================================================================
// Hook Return Type
// ============================================================================

export interface UseWebSocketReturn {
  /** Current connection state */
  connected: boolean;
  /** Currently connecting */
  connecting: boolean;
  /** Connection error if any */
  error: Error | null;
  /** Current reconnection attempt (0-based) */
  reconnectAttempt: number;
  /** Whether reconnection is in progress */
  reconnecting: boolean;
  /** Manually trigger reconnection */
  reconnect: () => void;
  /** Send a message through the WebSocket */
  sendMessage: (message: WsMessage) => void;
  /** Close the WebSocket connection */
  disconnect: () => void;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Calculate reconnection delay with exponential backoff
 */
function calculateReconnectDelay(attempt: number): number {
  return Math.min(
    WS_CONFIG.BASE_RECONNECT_DELAY * Math.pow(2, attempt),
    WS_CONFIG.MAX_RECONNECT_DELAY,
  );
}

/**
 * Get WebSocket URL with correct protocol
 */
function getWebSocketUrl(baseUrl: string): string {
  const url = new URL(baseUrl);
  const protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${url.host}${url.pathname}`;
}

// ============================================================================
// Main Hook
// ============================================================================

/**
 * WebSocket connection hook with automatic reconnection
 *
 * @param options - WebSocket configuration options
 * @returns WebSocket connection state and controls
 */
export function useWebSocket(options: UseWebSocketOptions): UseWebSocketReturn {
  const {
    url,
    token,
    roomName,
    onOpen,
    onMessage,
    onClose,
    onError,
    onReconnect,
    enableReconnect = true,
  } = options;

  // Refs to avoid re-renders and maintain stable references
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const heartbeatTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptRef = useRef(0);
  const isManualCloseRef = useRef(false);

  // State
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [reconnecting, setReconnecting] = useState(false);
  const [reconnectAttempt, setReconnectAttempt] = useState(0);

  /**
   * Send a message through the WebSocket
   */
  const sendMessage = useCallback((message: WsMessage) => {
    const ws = wsRef.current;
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message));
    } else {
      console.warn("WebSocket is not connected, cannot send message:", message);
    }
  }, []);

  /**
   * Send PONG in response to PING
   */
  const sendPong = useCallback(() => {
    sendMessage({
      message_type: WsMessageType.Pong,
      timestamp: Date.now(),
    });
  }, [sendMessage]);

  /**
   * Start heartbeat timeout
   */
  const startHeartbeat = useCallback(() => {
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current);
    }
    heartbeatTimeoutRef.current = setTimeout(() => {
      const ws = wsRef.current;
      if (ws?.readyState === WebSocket.OPEN) {
        ws.close();
      }
    }, WS_CONFIG.HEARTBEAT_INTERVAL + 5000); // Add 5s buffer
  }, []);

  /**
   * Stop heartbeat timeout
   */
  const stopHeartbeat = useCallback(() => {
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current);
      heartbeatTimeoutRef.current = null;
    }
  }, []);

  /**
   * Clear reconnection timeout
   */
  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
  }, []);

  /**
   * Connect to WebSocket server
   */
  const connect = useCallback(() => {
    // Don't connect if already connected or connecting
    if (wsRef.current?.readyState === WebSocket.OPEN || connecting) {
      return;
    }

    setConnecting(true);
    setError(null);

    try {
      const wsUrl = getWebSocketUrl(url);
      const ws = new WebSocket(wsUrl);

      wsRef.current = ws;

      ws.onopen = (event) => {
        setConnected(true);
        setConnecting(false);
        setReconnecting(false);
        setError(null);
        reconnectAttemptRef.current = 0;
        setReconnectAttempt(0);

        // Send CONNECT message
        const connectMessage: WsMessage = {
          message_type: WsMessageType.Connect,
          payload: {
            token,
            room_name: roomName,
          } as ConnectRequest,
          timestamp: Date.now(),
        };
        ws.send(JSON.stringify(connectMessage));

        onOpen?.(event);
      };

      ws.onmessage = (event) => {
        try {
          const message: WsMessage = JSON.parse(event.data);

          // Handle heartbeat
          if (message.message_type === WsMessageType.Ping) {
            sendPong();
            startHeartbeat();
            return;
          }

          // Handle connection acknowledgment
          if (message.message_type === WsMessageType.ConnectAck) {
            const ack = message.payload as ConnectAck;
            if (ack.success) {
              startHeartbeat();
            } else {
              setError(new Error(ack.message || "Connection failed"));
              ws.close();
            }
          }

          onMessage?.(message);
        } catch (err) {
          console.error("Failed to parse WebSocket message:", err);
        }
      };

      ws.onclose = (event) => {
        setConnected(false);
        setConnecting(false);
        stopHeartbeat();

        // Don't reconnect if manually closed
        if (isManualCloseRef.current) {
          isManualCloseRef.current = false;
          onClose?.(event);
          return;
        }

        // Attempt reconnection if enabled
        if (enableReconnect && reconnectAttemptRef.current < WS_CONFIG.MAX_RECONNECT_ATTEMPTS) {
          const delay = calculateReconnectDelay(reconnectAttemptRef.current);
          setReconnecting(true);

          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttemptRef.current += 1;
            setReconnectAttempt(reconnectAttemptRef.current);
            onReconnect?.(reconnectAttemptRef.current);
            connect();
          }, delay);
        } else {
          setError(new Error(`WebSocket closed: ${event.reason || "Unknown error"}`));
          onClose?.(event);
        }
      };

      ws.onerror = (event) => {
        setError(new Error("WebSocket error occurred"));
        onError?.(event);
      };
    } catch (err) {
      const errorObj = err instanceof Error ? err : new Error("Failed to create WebSocket");
      setError(errorObj);
      setConnecting(false);
    }
  }, [
    url,
    token,
    roomName,
    connecting,
    enableReconnect,
    onOpen,
    onMessage,
    onClose,
    onError,
    onReconnect,
    sendPong,
    startHeartbeat,
    stopHeartbeat,
  ]);

  /**
   * Manually trigger reconnection
   */
  const reconnect = useCallback(() => {
    clearReconnectTimeout();
    reconnectAttemptRef.current = 0;
    setReconnectAttempt(0);

    // Close existing connection if any
    if (wsRef.current) {
      isManualCloseRef.current = true;
      wsRef.current.close();
    }

    // Connect again
    connect();
  }, [connect, clearReconnectTimeout]);

  /**
   * Disconnect from WebSocket server
   */
  const disconnect = useCallback(() => {
    isManualCloseRef.current = true;
    clearReconnectTimeout();
    stopHeartbeat();

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setConnected(false);
    setConnecting(false);
    setReconnecting(false);
  }, [clearReconnectTimeout, stopHeartbeat]);

  /**
   * Effect: Manage connection lifecycle
   */
  useEffect(() => {
    connect();

    return () => {
      disconnect();
    };
  }, [url, token, roomName]); // Only reconnect when these change

  /**
   * Effect: Cleanup on unmount
   */
  useEffect(() => {
    return () => {
      clearReconnectTimeout();
      stopHeartbeat();
    };
  }, [clearReconnectTimeout, stopHeartbeat]);

  return {
    connected,
    connecting,
    error,
    reconnectAttempt,
    reconnecting,
    reconnect,
    sendMessage,
    disconnect,
  };
}
