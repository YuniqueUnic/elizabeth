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
import { useTranslations } from "next-intl";

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
  const t = useTranslations("room");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!password.trim()) {
      setError(t("passwordDialog.enterPassword"));
      return;
    }

    try {
      setLoading(true);
      setError(null);
      await onSubmit(password);
    } catch (err: any) {
      setError(err.message || t("passwordDialog.verifyFailed"));
    } finally {
      setLoading(false);
    }
  };

  const isPasswordValid = password.trim().length > 0;
  const canSubmit = isPasswordValid;

  return (
    <Dialog open={open} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
              <Lock className="h-5 w-5 text-primary" />
            </div>
            <div>
              <DialogTitle>{t("passwordDialog.title")}</DialogTitle>
              <DialogDescription className="mt-1">
                {t("passwordDialog.description", { roomName })}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="password">{t("passwordDialog.passwordLabel")}</Label>
              <div className="relative">
                <Input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    setError(null);
                  }}
                  placeholder={t("passwordDialog.passwordPlaceholder")}
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
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>

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
              {t("passwordDialog.cancel")}
            </Button>
            <Button
              type="submit"
              disabled={loading || !canSubmit}
            >
              {loading ? t("passwordDialog.verifying") : t("passwordDialog.enterRoom")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
