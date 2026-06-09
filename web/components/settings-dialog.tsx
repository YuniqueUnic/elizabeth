"use client";

import type React from "react";
import { useEffect } from "react";

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  desktopNotificationActionsByKind,
  desktopNotificationKinds,
  getBrowserNotificationPermission,
  requestBrowserNotificationPermission,
} from "@/lib/desktop-notifications";
import { useAppStore } from "@/lib/store";
import { useTranslations } from "next-intl";

type SettingsTab = "general" | "messages" | "notifications" | "appearance";

const settingsTabs: SettingsTab[] = [
  "general",
  "messages",
  "notifications",
  "appearance",
];

type SettingRowProps = {
  id: string;
  label: string;
  description: string;
  children: React.ReactNode;
};

function SettingRow({ id, label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-lg border border-border/70 px-4 py-3">
      <div className="min-w-0 flex-1 space-y-1">
        <Label htmlFor={id} className="text-sm font-medium leading-none">
          {label}
        </Label>
        <p className="text-sm leading-5 text-muted-foreground">
          {description}
        </p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

type FontSizeSettingProps = {
  id: string;
  label: string;
  description: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  onChange: (value: number) => void;
};

function FontSizeSetting({
  id,
  label,
  description,
  min,
  max,
  step = 1,
  value,
  onChange,
}: FontSizeSettingProps) {
  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const nextValue = Number.parseInt(event.target.value, 10);
    if (!Number.isNaN(nextValue) && nextValue >= min && nextValue <= max) {
      onChange(nextValue);
    }
  };

  return (
    <div className="space-y-3 rounded-lg border border-border/70 px-4 py-3">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-4">
        <Slider
          id={id}
          min={min}
          max={max}
          step={step}
          value={[value]}
          onValueChange={([nextValue]) => onChange(nextValue)}
          className="flex-1"
        />
        <Input
          aria-label={label}
          type="number"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={handleInputChange}
          className="w-20"
        />
      </div>
      <p className="text-sm text-muted-foreground">{description}</p>
    </div>
  );
}

export function SettingsDialog({ children }: { children: React.ReactNode }) {
  const t = useTranslations("settings");
  const {
    sendOnEnter,
    setSendOnEnter,
    includeMetadataInCopy,
    setIncludeMetadataInCopy,
    includeMetadataInDownload,
    setIncludeMetadataInDownload,
    editorFontSize,
    setEditorFontSize,
    toolbarButtonSize,
    setToolbarButtonSize,
    messageFontSize,
    setMessageFontSize,
    useHeti,
    setUseHeti,
    showDeleteConfirmation,
    setShowDeleteConfirmation,
    autoScroll,
    setAutoScroll,
    desktopNotificationsEnabled,
    setDesktopNotificationsEnabled,
    desktopNotificationShowContent,
    setDesktopNotificationShowContent,
    desktopNotificationPermission,
    setDesktopNotificationPermission,
    desktopNotificationTypes,
    setDesktopNotificationType,
  } = useAppStore();

  useEffect(() => {
    setDesktopNotificationPermission(getBrowserNotificationPermission());
  }, [setDesktopNotificationPermission]);

  const handleDesktopNotificationsChange = async (checked: boolean) => {
    if (!checked) {
      setDesktopNotificationsEnabled(false);
      setDesktopNotificationPermission(getBrowserNotificationPermission());
      return;
    }

    const permission = await requestBrowserNotificationPermission();
    setDesktopNotificationPermission(permission);
    setDesktopNotificationsEnabled(permission === "granted");
  };

  const notificationPermissionLabel =
    desktopNotificationPermission === "granted"
      ? t("desktopNotifications.permission.granted")
      : desktopNotificationPermission === "denied"
        ? t("desktopNotifications.permission.denied")
        : desktopNotificationPermission === "unsupported"
          ? t("desktopNotifications.permission.unsupported")
          : t("desktopNotifications.permission.default");

  const notificationTypesDisabled =
    !desktopNotificationsEnabled || desktopNotificationPermission !== "granted";

  return (
    <Dialog>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent
        className="flex h-[calc(100dvh-1rem)] max-h-[720px] flex-col gap-0 overflow-hidden p-0 sm:h-[min(720px,calc(100dvh-2rem))] sm:max-w-2xl"
        data-testid="settings-dialog"
      >
        <DialogHeader className="shrink-0 px-6 pt-6 pb-4 pr-12">
          <DialogTitle>{t("title")}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>

        <Tabs
          defaultValue="general"
          className="min-h-0 flex-1 gap-0"
          data-testid="settings-tabs"
        >
          <div className="shrink-0 overflow-x-auto border-y border-border/70 px-6 py-3">
            <TabsList className="h-auto min-w-full justify-start rounded-md">
              {settingsTabs.map((tab) => (
                <TabsTrigger
                  key={tab}
                  value={tab}
                  className="px-3"
                  data-testid={`settings-tab-${tab}`}
                >
                  {t(`tabs.${tab}`)}
                </TabsTrigger>
              ))}
            </TabsList>
          </div>

          <div
            className="min-h-0 flex-1 overflow-y-auto px-6 py-4"
            data-testid="settings-dialog-scroll"
          >
            <TabsContent
              value="general"
              className="mt-0 space-y-4"
              data-testid="settings-tab-general-panel"
            >
              <SettingRow
                id="send-on-enter"
                label={t("sendOnEnter.label")}
                description={
                  sendOnEnter
                    ? t("sendOnEnter.enabled")
                    : t("sendOnEnter.disabled")
                }
              >
                <Switch
                  id="send-on-enter"
                  checked={sendOnEnter}
                  onCheckedChange={setSendOnEnter}
                />
              </SettingRow>

              <SettingRow
                id="use-heti"
                label={t("useHeti.label")}
                description={t("useHeti.description")}
              >
                <Switch
                  id="use-heti"
                  checked={useHeti}
                  onCheckedChange={setUseHeti}
                />
              </SettingRow>

              <SettingRow
                id="delete-confirmation"
                label={t("showDeleteConfirmation.label")}
                description={t("showDeleteConfirmation.description")}
              >
                <Switch
                  id="delete-confirmation"
                  checked={showDeleteConfirmation}
                  onCheckedChange={setShowDeleteConfirmation}
                  data-testid="setting-delete-confirmation"
                />
              </SettingRow>

              <SettingRow
                id="auto-scroll"
                label={t("autoScroll.label")}
                description={t("autoScroll.description")}
              >
                <Switch
                  id="auto-scroll"
                  checked={autoScroll}
                  onCheckedChange={setAutoScroll}
                  data-testid="setting-auto-scroll"
                />
              </SettingRow>
            </TabsContent>

            <TabsContent
              value="messages"
              className="mt-0 space-y-4"
              data-testid="settings-tab-messages-panel"
            >
              <SettingRow
                id="include-metadata-copy"
                label={t("includeMetadataInCopy.label")}
                description={t("includeMetadataInCopy.description")}
              >
                <Switch
                  id="include-metadata-copy"
                  checked={includeMetadataInCopy}
                  onCheckedChange={setIncludeMetadataInCopy}
                  data-testid="setting-include-metadata-copy"
                />
              </SettingRow>

              <SettingRow
                id="include-metadata-download"
                label={t("includeMetadataInDownload.label")}
                description={t("includeMetadataInDownload.description")}
              >
                <Switch
                  id="include-metadata-download"
                  checked={includeMetadataInDownload}
                  onCheckedChange={setIncludeMetadataInDownload}
                  data-testid="setting-include-metadata-download"
                />
              </SettingRow>
            </TabsContent>

            <TabsContent
              value="notifications"
              className="mt-0 space-y-4"
              data-testid="settings-tab-notifications-panel"
            >
              <SettingRow
                id="desktop-notifications"
                label={t("desktopNotifications.label")}
                description={t("desktopNotifications.description", {
                  permission: notificationPermissionLabel,
                })}
              >
                <Switch
                  id="desktop-notifications"
                  checked={
                    desktopNotificationsEnabled &&
                    desktopNotificationPermission === "granted"
                  }
                  onCheckedChange={handleDesktopNotificationsChange}
                  disabled={desktopNotificationPermission === "unsupported"}
                  data-testid="setting-desktop-notifications"
                />
              </SettingRow>

              <SettingRow
                id="desktop-notification-show-content"
                label={t("desktopNotifications.showContent.label")}
                description={t("desktopNotifications.showContent.description")}
              >
                <Switch
                  id="desktop-notification-show-content"
                  checked={desktopNotificationShowContent}
                  onCheckedChange={setDesktopNotificationShowContent}
                  disabled={notificationTypesDisabled}
                  data-testid="setting-desktop-notification-show-content"
                />
              </SettingRow>

              <div
                className="rounded-lg border border-border/70 px-4"
                data-testid="settings-notification-accordion"
              >
                <Accordion type="multiple" defaultValue={["message"]}>
                  {desktopNotificationKinds.map((kind) => (
                    <AccordionItem key={kind} value={kind}>
                      <AccordionTrigger
                        data-testid={`settings-notification-${kind}-trigger`}
                      >
                        <span className="min-w-0">
                          <span className="block">
                            {t(`desktopNotifications.kinds.${kind}`)}
                          </span>
                          <span className="mt-1 block text-xs font-normal leading-4 text-muted-foreground">
                            {t(`desktopNotifications.kindDescriptions.${kind}`)}
                          </span>
                        </span>
                      </AccordionTrigger>
                      <AccordionContent>
                        <div className="grid gap-2 sm:grid-cols-3">
                          {desktopNotificationActionsByKind[kind].map((action) => {
                            const switchId =
                              `desktop-notification-${kind}-${action}`;

                            return (
                              <div
                                key={`${kind}-${action}`}
                                className="flex items-center justify-between gap-3 rounded-md border border-border/60 px-3 py-2"
                              >
                                <Label
                                  htmlFor={switchId}
                                  className="text-sm font-normal"
                                >
                                  {t(`desktopNotifications.actions.${action}`)}
                                </Label>
                                <Switch
                                  id={switchId}
                                  checked={desktopNotificationTypes[kind][action]}
                                  onCheckedChange={(value) =>
                                    setDesktopNotificationType(
                                      kind,
                                      action,
                                      value,
                                    )}
                                  disabled={notificationTypesDisabled}
                                  data-testid={`setting-desktop-notification-${kind}-${action}`}
                                />
                              </div>
                            );
                          })}
                        </div>
                      </AccordionContent>
                    </AccordionItem>
                  ))}
                </Accordion>
              </div>
            </TabsContent>

            <TabsContent
              value="appearance"
              className="mt-0 space-y-4"
              data-testid="settings-tab-appearance-panel"
            >
              <FontSizeSetting
                id="editor-font-size"
                label={t("editorFontSize.label")}
                description={t("editorFontSize.description")}
                min={4}
                max={64}
                value={editorFontSize}
                onChange={setEditorFontSize}
              />

              <FontSizeSetting
                id="toolbar-button-size"
                label={t("toolbarButtonSize.label")}
                description={t("toolbarButtonSize.description")}
                min={20}
                max={36}
                step={2}
                value={toolbarButtonSize}
                onChange={setToolbarButtonSize}
              />

              <FontSizeSetting
                id="message-font-size"
                label={t("messageFontSize.label")}
                description={t("messageFontSize.description")}
                min={4}
                max={64}
                value={messageFontSize}
                onChange={setMessageFontSize}
              />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
