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

  const redeemMutation = useMutation({
    mutationFn: () => redeemCode(roomName, contentId, { code }),
    onSuccess: (data) => {
      onSuccess(data.ticket);
      onOpenChange(false);
      setCode("");
    },
    onError: () => {
      toast({ title: t("redeemFailed"), variant: "destructive" });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!code.trim()) return;
    redeemMutation.mutate();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("redeemTitle")}</DialogTitle>
          <DialogDescription>{t("redeemDescription")}</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 py-4">
          <div className="space-y-2">
            <Label>{t("accessCode")}</Label>
            <Input
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder={t("accessCodePlaceholder")}
              autoFocus
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              {t("cancel")}
            </Button>
            <Button
              type="submit"
              disabled={!code.trim() || redeemMutation.isPending}
            >
              {t("redeemSubmit")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
