"use client";

import { useState } from "react";
import { Eye, EyeOff, Lock } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";

interface RoomPasswordDialogProps {
  roomName: string;
  open: boolean;
  onSubmit: (password: string) => Promise<void>;
  onCancel: () => void;
}

export function RoomPasswordDialog({
  roomName,
  open,
  onSubmit,
  onCancel,
}: RoomPasswordDialogProps) {
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!password.trim()) {
      setError("请输入密码");
      return;
    }

    try {
      setLoading(true);
      setError(null);
      await onSubmit(password);
    } catch (err: any) {
      setError(err.message || "密码验证失败");
    } finally {
      setLoading(false);
    }
  };

  // 添加一个确认密码状态和第二个密码输入框
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  const isPasswordValid = password.trim().length > 0;
  const isPasswordMatch = password === confirmPassword;

  // 只有在显示密码时才需要验证确认密码
  const needsConfirmPassword = showPassword || showConfirmPassword;
  const canSubmit = isPasswordValid && (!needsConfirmPassword || isPasswordMatch);

  return (
    <Dialog open={open} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
              <Lock className="h-5 w-5 text-primary" />
            </div>
            <div>
              <DialogTitle>房间已加密</DialogTitle>
              <DialogDescription className="mt-1">
                房间{" "}
                <span className="font-medium text-foreground">{roomName}</span>
                {" "}
                需要密码才能访问
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            {/* 只有在显示密码时才显示密码输入 */}
            {showPassword || showConfirmPassword ? (
              <>
                <div className="space-y-2">
                  <Label htmlFor="password">密码</Label>
                  <div className="relative">
                    <Input
                      id="password"
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(e) => {
                        setPassword(e.target.value);
                        setError(null);
                      }}
                      placeholder="请输入房间密码"
                      disabled={loading}
                      autoFocus
                      className="pr-10"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="absolute right-0 top-0 h-full"
                      onClick={() => setShowPassword(!showPassword)}
                      disabled={loading}
                      tabIndex={-1}
                    >
                      {showPassword
                        ? <EyeOff className="h-4 w-4" />
                        : <Eye className="h-4 w-4" />}
                    </Button>
                  </div>
                </div>

                {/* 如果显示了密码，则显示确认密码框 */}
                <div className="space-y-2">
                  <Label htmlFor="confirm-password">确认密码</Label>
                  <div className="relative">
                    <Input
                      id="confirm-password"
                      type={showConfirmPassword ? "text" : "password"}
                      value={confirmPassword}
                      onChange={(e) => {
                        setConfirmPassword(e.target.value);
                        setError(null);
                      }}
                      placeholder="请再次输入密码"
                      disabled={loading}
                      className="pr-10"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="absolute right-0 top-0 h-full"
                      onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                      disabled={loading}
                      tabIndex={-1}
                    >
                      {showConfirmPassword
                        ? <EyeOff className="h-4 w-4" />
                        : <Eye className="h-4 w-4" />}
                    </Button>
                  </div>
                </div>

                {error && (
                  <Alert variant="destructive">
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}
              </>
            ) : (
              /* 隐藏密码状态，只显示两个密码框 */
              <>
                <div className="space-y-2">
                  <Label htmlFor="password">密码</Label>
                  <div className="relative">
                    <Input
                      id="password"
                      type="password"
                      value={password}
                      onChange={(e) => {
                        setPassword(e.target.value);
                        setError(null);
                      }}
                      placeholder="请输入房间密码"
                      disabled={loading}
                      autoFocus
                      className="pr-10"
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="confirm-password">确认密码</Label>
                  <div className="relative">
                    <Input
                      id="confirm-password"
                      type="password"
                      value={confirmPassword}
                      onChange={(e) => {
                        setConfirmPassword(e.target.value);
                        setError(null);
                      }}
                      placeholder="请再次输入密码"
                      disabled={loading}
                      className="pr-10"
                    />
                  </div>
                </div>

                {error && (
                  <Alert variant="destructive">
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}
              </>
            )}

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              type="button"
              variant="outline"
              onClick={onCancel}
              disabled={loading}
            >
              取消
            </Button>
            <Button
              type="submit"
              disabled={loading || !canSubmit}
            >
              {loading ? "验证中..." : "进入房间"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
