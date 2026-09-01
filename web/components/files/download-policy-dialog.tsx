"use client";

import { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { getPolicy, setPolicy, generateCodes, type PolicyMode } from "@/api/policyService";
import { useToast } from "@/hooks/use-toast";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { Copy } from "lucide-react";

interface DownloadPolicyDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  roomName: string;
  contentId: string;
}

export function DownloadPolicyDialog({
  open,
  onOpenChange,
  roomName,
  contentId,
}: DownloadPolicyDialogProps) {
  const t = useTranslations("room.downloadPolicy");
  const { toast } = useToast();
  const queryClient = useQueryClient();

  const [mode, setMode] = useState<PolicyMode>("off");
  const [maxDownloads, setMaxDownloads] = useState<number>(0);
  const [codeCount, setCodeCount] = useState<number>(1);
  const [generatedCodes, setGeneratedCodes] = useState<string[]>([]);

  const { data: policy, isLoading } = useQuery({
    queryKey: ["policy", roomName, contentId],
    queryFn: () => getPolicy(roomName, contentId),
    enabled: open && !!roomName && !!contentId,
  });

  useEffect(() => {
    if (policy) {
      setMode(policy.mode);
      setMaxDownloads(policy.max_downloads || 0);
    }
  }, [policy]);

  const saveMutation = useMutation({
    mutationFn: () =>
      setPolicy(roomName, contentId, {
        mode,
        max_downloads: maxDownloads > 0 ? maxDownloads : undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["policy", roomName, contentId] });
      toast({ title: t("saveSuccess") });
      onOpenChange(false);
    },
    onError: () => {
      toast({ title: t("saveFailed"), variant: "destructive" });
    },
  });

  const generateMutation = useMutation({
    mutationFn: () =>
      generateCodes(roomName, contentId, {
        count: mode === "one_time" ? codeCount : 1,
        is_reusable: mode === "reusable",
      }),
    onSuccess: (data) => {
      setGeneratedCodes(data.codes);
      queryClient.invalidateQueries({ queryKey: ["policy", roomName, contentId] });
      toast({ title: t("generateSuccess") });
    },
    onError: () => {
      toast({ title: t("generateFailed"), variant: "destructive" });
    },
  });

  const handleCopyCodes = async () => {
    try {
      await copyTextToClipboard(generatedCodes.join("\\n"));
      toast({ title: t("copied") });
    } catch {
      toast({ title: t("copyFailed"), variant: "destructive" });
    }
  };

  const handleSave = () => {
    saveMutation.mutate();
  };

  const handleGenerate = () => {
    generateMutation.mutate();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-4 text-center text-sm text-muted-foreground">{t("loading")}</div>
        ) : (
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>{t("mode")}</Label>
              <Select value={mode} onValueChange={(v) => setMode(v as PolicyMode)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="off">{t("modeOff")}</SelectItem>
                  <SelectItem value="reusable">{t("modeReusable")}</SelectItem>
                  <SelectItem value="one_time">{t("modeOneTime")}</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {(mode === "off" || mode === "reusable") && (
              <div className="space-y-2">
                <Label>{t("maxDownloads")}</Label>
                <Input
                  type="number"
                  min={0}
                  value={maxDownloads}
                  onChange={(e) => setMaxDownloads(parseInt(e.target.value) || 0)}
                  placeholder={t("maxDownloadsPlaceholder")}
                />
                <p className="text-xs text-muted-foreground">{t("maxDownloadsHint")}</p>
              </div>
            )}

            {mode === "one_time" && (
              <div className="space-y-2">
                <Label>{t("codeCount")}</Label>
                <Input
                  type="number"
                  min={1}
                  max={100}
                  value={codeCount}
                  onChange={(e) => setCodeCount(parseInt(e.target.value) || 1)}
                />
              </div>
            )}

            {policy?.mode !== "off" && mode === policy?.mode && (
              <div className="pt-4 border-t space-y-2">
                <div className="flex items-center justify-between">
                  <Label>{t("generateCodes")}</Label>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleGenerate}
                    disabled={generateMutation.isPending}
                  >
                    {t("generateBtn")}
                  </Button>
                </div>
                {policy?.mode === "one_time" && policy.pool_size !== undefined && (
                  <p className="text-xs text-muted-foreground">
                    {t("poolSize", { size: policy.pool_size })}
                  </p>
                )}
                {generatedCodes.length > 0 && (
                  <div className="p-3 bg-muted rounded-md space-y-2 relative">
                    <p className="text-xs font-medium text-destructive mb-2">{t("generateWarning")}</p>
                    <div className="max-h-32 overflow-y-auto text-sm space-y-1 select-all break-all pr-8">
                      {generatedCodes.map((c, i) => (
                        <div key={i}>{c}</div>
                      ))}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="absolute right-1 bottom-1 h-6 w-6"
                      onClick={handleCopyCodes}
                    >
                      <Copy className="h-3 w-3" />
                    </Button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("cancel")}
          </Button>
          <Button onClick={handleSave} disabled={saveMutation.isPending}>
            {t("save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
