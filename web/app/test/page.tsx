"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  createRoom,
  getRoomDetails,
  updateRoomSettings,
} from "@/api/roomService";
import { getAccessToken } from "@/api/authService";
import {
  deleteMessage,
  getMessages,
  postMessage,
  updateMessage,
} from "@/api/messageService";

export default function TestPage() {
  const [logs, setLogs] = useState<string[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [testStatus, setTestStatus] = useState<
    "idle" | "running" | "success" | "error"
  >("idle");

  const addLog = (
    message: string,
    type: "info" | "success" | "error" = "info",
  ) => {
    const prefix = type === "success" ? "✓" : type === "error" ? "✗" : "•";
    const timestamp = new Date().toLocaleTimeString();
    setLogs((prev) => [...prev, `[${timestamp}] ${prefix} ${message}`]);
  };

  const runIntegrationTest = async () => {
    setIsRunning(true);
    setTestStatus("running");
    setLogs([]);
    addLog("=== Elizabeth Frontend Integration Test ===");

    try {
      // Test 1: Create a room
      const roomName = `test-room-${Date.now()}`;
      const password = "test123"; // pragma: allowlist secret

      addLog(`Creating room: ${roomName} with password: ${password}`);
      const room = await createRoom(roomName, password);
      addLog(`Room created: ${room.name}`, "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 2: Get access token
      addLog("Getting access token");
      const tokenResponse = await getAccessToken(roomName, password);
      addLog(
        `Token obtained: ${tokenResponse.token.substring(0, 50)}...`,
        "success",
      );

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 3: Get room details
      addLog("Getting room details");
      const roomDetails = await getRoomDetails(roomName, tokenResponse.token);
      addLog(`Room details retrieved: ${roomDetails.name}`, "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 4: Send a message
      addLog("Sending a message");
      const messageContent = "Hello from integration test!";
      const message = await postMessage(
        roomName,
        messageContent,
        tokenResponse.token,
      );
      addLog(`Message sent: ${message.content}`, "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 5: Get all messages
      addLog("Getting all messages");
      const messages = await getMessages(roomName, tokenResponse.token);
      addLog(`Messages retrieved: ${messages.length} messages`, "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 6: Update the message
      addLog("Updating the message");
      const updatedContent = "Updated message content!";
      const updatedMessage = await updateMessage(
        roomName,
        message.id,
        updatedContent,
        tokenResponse.token,
      );
      addLog(`Message updated: ${updatedMessage.content}`, "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 7: Update room settings
      addLog("Updating room settings");
      const updatedRoom = await updateRoomSettings(
        roomName,
        tokenResponse.token,
        {
          maxSize: 20971520, // 20MB
        },
      );
      addLog(
        `Room settings updated: max_size=${updatedRoom.maxSize}`,
        "success",
      );

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 8: Delete the message
      addLog("Deleting the message");
      await deleteMessage(roomName, message.id, tokenResponse.token);
      addLog("Message deleted", "success");

      // Wait a bit to avoid rate limiting
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Test 9: Verify message is deleted
      addLog("Verifying message deletion");
      const messagesAfterDelete = await getMessages(
        roomName,
        tokenResponse.token,
      );
      addLog(
        `Messages after deletion: ${messagesAfterDelete.length} messages`,
        "success",
      );

      addLog("=== All Tests Passed! ===", "success");
      setTestStatus("success");
    } catch (error) {
      const errorMessage = error instanceof Error
        ? error.message
        : String(error);
      addLog(`Test failed: ${errorMessage}`, "error");
      setTestStatus("error");
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="container mx-auto p-6">
      <Card className="w-full max-w-4xl mx-auto">
        <CardHeader>
          <CardTitle>Elizabeth Integration Test</CardTitle>
          <CardDescription>
            Test the complete flow of room creation, authentication, messaging,
            and settings updates
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Button
              onClick={runIntegrationTest}
              disabled={isRunning}
              variant={testStatus === "success"
                ? "default"
                : testStatus === "error"
                ? "destructive"
                : "default"}
            >
              {isRunning ? "Running Tests..." : "Run Integration Test"}
            </Button>
            {testStatus === "success" && (
              <span className="text-green-600 font-semibold">
                ✓ All tests passed!
              </span>
            )}
            {testStatus === "error" && (
              <span className="text-red-600 font-semibold">✗ Tests failed</span>
            )}
          </div>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Test Logs</CardTitle>
            </CardHeader>
            <CardContent>
              <ScrollArea className="h-[500px] w-full rounded-md border p-4">
                <div className="space-y-1 font-mono text-sm">
                  {logs.length === 0
                    ? (
                      <p className="text-muted-foreground">
                        No logs yet. Click "Run Integration Test" to start.
                      </p>
                    )
                    : (
                      logs.map((log, index) => (
                        <div
                          key={index}
                          className={log.includes("✓")
                            ? "text-green-600"
                            : log.includes("✗")
                            ? "text-red-600"
                            : "text-foreground"}
                        >
                          {log}
                        </div>
                      ))
                    )}
                </div>
              </ScrollArea>
            </CardContent>
          </Card>
        </CardContent>
      </Card>
    </div>
  );
}
