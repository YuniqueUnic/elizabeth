"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation } from "@tanstack/react-query";
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
import { redeemCode } from "@/api/policyService";
import { useToast } from "@/hooks/use-toast";
import { Lock, AlertCircle } from "lucide-react";
import { APIError } from "@/lib/utils/api";

interface RedeemDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  roomName: string;
  contentId: string;
  onSuccess: (ticket: string) => void;
}

export function RedeemDialog({
  open,
  onOpenChange,
  roomName,
  contentId,
  onSuccess,
}: RedeemDialogProps) {
  const t = useTranslations("room.downloadPolicy");
  const { toast } = useToast();
  const [code, setCode] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const redeemMutation = useMutation({
    mutationFn: () => redeemCode(roomName, contentId, { code: code.trim() }),
    onSuccess: (data) => {
      onSuccess(data.ticket);
      onOpenChange(false);
      setCode("");
      setErrorMessage(null);
    },
    onError: (error: unknown) => {
      let msg = t("redeemFailed");
      if (error instanceof APIError) {
        if (error.code === 429) {
          msg = error.message || t("rateLimited");
        } else if (error.message) {
          msg = error.message;
        }
      } else if (error instanceof Error && error.message) {
        msg = error.message;
      }
      setErrorMessage(msg);
      toast({ title: msg, variant: "destructive" });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!code.trim()) return;
    setErrorMessage(null);
    redeemMutation.mutate();
  };

  const handleOpenChange = (isOpen: boolean) => {
    if (!isOpen) {
      setCode("");
      setErrorMessage(null);
    }
    onOpenChange(isOpen);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Lock className="h-5 w-5 text-primary" />
            {t("redeemTitle")}
          </DialogTitle>
          <DialogDescription>{t("redeemDescription")}</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 py-2">
          <div className="space-y-2">
            <Label>{t("accessCode")}</Label>
            <Input
              value={code}
              onChange={(e) => {
                setCode(e.target.value);
                if (errorMessage) setErrorMessage(null);
              }}
              placeholder={t("accessCodePlaceholder")}
              autoFocus
              disabled={redeemMutation.isPending}
            />
            {errorMessage && (
              <p className="text-xs font-medium text-destructive flex items-center gap-1 mt-1">
                <AlertCircle className="h-3.5 w-3.5 shrink-0" />
                <span>{errorMessage}</span>
              </p>
            )}
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              type="button"
              variant="outline"
              onClick={() => handleOpenChange(false)}
              disabled={redeemMutation.isPending}
            >
              {t("cancel")}
            </Button>
            <Button
              type="submit"
              disabled={!code.trim() || redeemMutation.isPending}
            >
              {redeemMutation.isPending ? t("redeeming") : t("redeemSubmit")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
