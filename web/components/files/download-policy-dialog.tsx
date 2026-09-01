"use client";

import { useEffect, useState, useMemo } from "react";
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
import { Textarea } from "@/components/ui/textarea";
import { getPolicy, setPolicy, type PolicyMode } from "@/api/policyService";
import { useToast } from "@/hooks/use-toast";
import { copyTextToClipboard } from "@/lib/utils/clipboard";
import { Copy } from "lucide-react";
import { Badge } from "@/components/ui/badge";

interface DownloadPolicyDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  roomName: string;
  contentId: string;
}

function generateRandomCode(length: number): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  let result = "";
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
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
  
  // Download limits
  const [maxDownloadsLimit, setMaxDownloadsLimit] = useState<string>("unlimited");
  const [customLimit, setCustomLimit] = useState<number>(0);
  
  // Reusable
  const [reusableCode, setReusableCode] = useState<string>("");
  
  // One time
  const [batchCount, setBatchCount] = useState<number>(10);
  const [oneTimeCodes, setOneTimeCodes] = useState<string>("");

  const { data: policy, isLoading } = useQuery({
    queryKey: ["policy", roomName, contentId],
    queryFn: () => getPolicy(roomName, contentId),
    enabled: open && !!roomName && !!contentId,
    staleTime: 30_000,
    retry: 1,
  });

  const policyLoaded = !isLoading;

  useEffect(() => {
    if (policyLoaded && policy) {
      setMode(policy.mode);
      if (policy.max_downloads === null || policy.max_downloads === undefined) {
        setMaxDownloadsLimit("unlimited");
      } else if ([1, 10, 100].includes(policy.max_downloads)) {
        setMaxDownloadsLimit(policy.max_downloads.toString());
      } else {
        setMaxDownloadsLimit("custom");
        setCustomLimit(policy.max_downloads);
      }
    } else if (policyLoaded && !policy) {
      setMode("off");
      setMaxDownloadsLimit("unlimited");
    }
  }, [policyLoaded, policy, open]);

  const validOneTimeCodes = useMemo(() => {
    return oneTimeCodes.split("\n").map(s => s.trim()).filter(s => s.length > 0);
  }, [oneTimeCodes]);

  const saveMutation = useMutation({
    mutationFn: () => {
      let max_downloads: number | null = null;
      if (mode !== "one_time") {
        if (maxDownloadsLimit === "custom") {
          max_downloads = customLimit;
        } else if (maxDownloadsLimit !== "unlimited") {
          max_downloads = parseInt(maxDownloadsLimit, 10);
        }
      }

      let codes: string[] | undefined = undefined;
      if (mode === "reusable" && reusableCode.trim()) {
        codes = [reusableCode.trim()];
      } else if (mode === "one_time" && validOneTimeCodes.length > 0) {
        codes = validOneTimeCodes;
      }

      return setPolicy(roomName, contentId, {
        mode,
        max_downloads,
        codes,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["policy", roomName, contentId] });
      toast({ title: t("saveSuccess") });
      onOpenChange(false);
    },
    onError: () => {
      toast({ title: t("saveFailed"), variant: "destructive" });
    },
  });

  const handleCopyCodes = async (text: string) => {
    try {
      await copyTextToClipboard(text);
      toast({ title: t("copied") });
    } catch {
      toast({ title: t("copyFailed"), variant: "destructive" });
    }
  };

  const handleExportTxt = () => {
    const blob = new Blob([oneTimeCodes], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `codes_${contentId}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleBatchGenerate = () => {
    const newCodes = Array.from({ length: batchCount }, () => generateRandomCode(8));
    setOneTimeCodes(prev => {
      const p = prev.trim();
      return p ? p + "\n" + newCodes.join("\n") : newCodes.join("\n");
    });
  };

  const handleDeduplicate = () => {
    const unique = Array.from(new Set(validOneTimeCodes));
    setOneTimeCodes(unique.join("\n"));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-4 text-center text-sm text-muted-foreground">{t("loading")}</div>
        ) : (
          <div className="space-y-4 py-4">
            
            {policy && policy.mode !== "off" && (
              <div className="bg-muted p-3 rounded-md space-y-2">
                <div className="flex items-center gap-2">
                  <Badge variant="outline">
                    {policy.mode === "reusable" ? t("modeReusable") : t("modeOneTime")}
                  </Badge>
                  <span className="text-sm">
                    {t("downloadStats", { 
                      count: policy.download_count, 
                      max: policy.max_downloads ?? t("unlimited") 
                    })}
                  </span>
                </div>
                <div className="text-sm text-muted-foreground">
                  {t("poolStats", { 
                    total: policy.total_codes, 
                    remaining: policy.remaining_codes, 
                    used: policy.used_codes 
                  })}
                </div>
              </div>
            )}

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
                <Label>{t("downloadLimit")}</Label>
                <Select value={maxDownloadsLimit} onValueChange={setMaxDownloadsLimit}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="unlimited">{t("limitUnlimited")}</SelectItem>
                    <SelectItem value="1">{t("limit1")}</SelectItem>
                    <SelectItem value="10">{t("limit10")}</SelectItem>
                    <SelectItem value="100">{t("limit100")}</SelectItem>
                    <SelectItem value="custom">{t("limitCustom")}</SelectItem>
                  </SelectContent>
                </Select>
                {maxDownloadsLimit === "custom" && (
                  <Input
                    type="number"
                    min={1}
                    value={customLimit}
                    onChange={(e) => setCustomLimit(parseInt(e.target.value) || 0)}
                    placeholder={t("customLimitPlaceholder")}
                    className="mt-2"
                  />
                )}
              </div>
            )}

            {mode === "reusable" && (
              <div className="space-y-2">
                <Label>{t("reusableCode")}</Label>
                <div className="flex gap-2">
                  <Input 
                    value={reusableCode} 
                    onChange={(e) => setReusableCode(e.target.value)} 
                    placeholder={t("reusableCodePlaceholder")} 
                  />
                  <Button variant="outline" onClick={() => setReusableCode(generateRandomCode(8))}>
                    {t("generateRandom")}
                  </Button>
                  <Button variant="ghost" size="icon" onClick={() => handleCopyCodes(reusableCode)}>
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            )}

            {mode === "one_time" && (
              <div className="space-y-2">
                <div className="flex flex-wrap gap-2 items-center">
                  <Input
                    type="number"
                    min={1}
                    max={1000}
                    value={batchCount}
                    onChange={(e) => setBatchCount(parseInt(e.target.value) || 1)}
                    className="w-20"
                  />
                  <Button variant="secondary" size="sm" onClick={handleBatchGenerate}>
                    {t("batchGenerate")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={handleDeduplicate}>
                    {t("deduplicate")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => setOneTimeCodes("")}>
                    {t("clear")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => handleCopyCodes(oneTimeCodes)}>
                    {t("copyAll")}
                  </Button>
                  <Button variant="outline" size="sm" onClick={handleExportTxt}>
                    {t("exportTxt")}
                  </Button>
                </div>
                <Textarea 
                  value={oneTimeCodes} 
                  onChange={(e) => setOneTimeCodes(e.target.value)} 
                  rows={6}
                  placeholder={t("oneTimeCodesPlaceholder")}
                />
                <div className="flex justify-between items-center text-xs">
                  <span className={validOneTimeCodes.length > 1000 ? "text-destructive font-medium" : "text-muted-foreground"}>
                    {t("validCodesCount", { count: validOneTimeCodes.length, max: 1000 })}
                  </span>
                  <span className="text-muted-foreground">
                    {t("autoDownloadTimesHint", { count: validOneTimeCodes.length })}
                  </span>
                </div>
              </div>
            )}
            
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("cancel")}
          </Button>
          <Button onClick={() => saveMutation.mutate()} disabled={saveMutation.isPending}>
            {t("save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
