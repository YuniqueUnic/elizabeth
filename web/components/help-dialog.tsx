"use client";

import type React from "react";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
    BookCopy,
    Clock,
    Copy,
    Download,
    Eye,
    HelpCircle,
    Icon,
    KeyRound,
    Lock,
    Pencil,
    QrCode,
    Save,
    Settings,
    Share2,
    Trash2,
} from "lucide-react";
import { useTranslations } from "next-intl";

interface HelpSectionProps {
    title: string;
    icon: React.ElementType;
    children: React.ReactNode;
}

const HelpSection: React.FC<HelpSectionProps> = ({
    title,
    icon: Icon,
    children,
}) => (
    <section>
        <h3 className="text-lg font-semibold mb-4 flex items-center">
            <Icon className="w-5 h-5 mr-2" />
            {title}
        </h3>
        <div className="space-y-4 pl-7">{children}</div>
    </section>
);

interface HelpItemProps {
    title: string;
    icon: React.ElementType;
    children: React.ReactNode;
}

const HelpItem: React.FC<HelpItemProps> = ({ title, icon: Icon, children }) => (
    <div>
        <h4 className="font-medium mb-1 flex items-center">
            <Icon className="w-4 h-4 mr-2 text-muted-foreground" />
            {title}
        </h4>
        <div className="text-sm text-muted-foreground pl-6">{children}</div>
    </div>
);

export function HelpDialog({ children }: { children: React.ReactNode }) {
    const t = useTranslations("help");

    return (
        <Dialog>
            <DialogTrigger asChild>{children}</DialogTrigger>
            <DialogContent className="sm:max-w-3xl max-h-[90vh] flex flex-col overflow-hidden">
                <DialogHeader className="shrink-0">
                    <DialogTitle>{t("title")}</DialogTitle>
                    <DialogDescription>
                        {t("description")}
                    </DialogDescription>
                </DialogHeader>

                <ScrollArea className="flex-1 pr-4 -mr-4 overflow-y-auto">
                    <div className="space-y-8 py-4 pr-4">
                        <HelpSection title={t("sections.toolbar.title")} icon={Settings}>
                            <HelpItem title={t("sections.toolbar.copyButton.title")} icon={Copy}>
                                {t("sections.toolbar.copyButton.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.toolbar.downloadButton.title")} icon={Download}>
                                {t("sections.toolbar.downloadButton.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.toolbar.saveButton.title")} icon={Save}>
                                {t("sections.toolbar.saveButton.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.toolbar.deleteButton.title")} icon={Trash2}>
                                {t("sections.toolbar.deleteButton.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.toolbar.helpButton.title")} icon={HelpCircle}>
                                {t("sections.toolbar.helpButton.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.toolbar.settingsButton.title")} icon={Settings}>
                                {t("sections.toolbar.settingsButton.description")}
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title={t("sections.roomSettings.title")} icon={Settings}>
                            <HelpItem title={t("sections.roomSettings.expiry.title")} icon={Clock}>
                                {t("sections.roomSettings.expiry.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.roomSettings.password.title")} icon={Lock}>
                                {t("sections.roomSettings.password.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.roomSettings.maxViews.title")} icon={Eye}>
                                {t("sections.roomSettings.maxViews.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.roomSettings.permissions.title")} icon={KeyRound}>
                                {t("sections.roomSettings.permissions.description")}
                                <ul className="list-disc list-inside mt-2 space-y-1">
                                    <li>
                                        <strong>{t("sections.roomSettings.permissions.items.preview.label")}</strong>：{t("sections.roomSettings.permissions.items.preview.description")}
                                    </li>
                                    <li>
                                        <strong>{t("sections.roomSettings.permissions.items.edit.label")}</strong>
                                        ：{t("sections.roomSettings.permissions.items.edit.description")}
                                    </li>
                                    <li>
                                        <strong>{t("sections.roomSettings.permissions.items.share.label")}</strong>
                                        ：{t("sections.roomSettings.permissions.items.share.description")}
                                    </li>
                                    <li>
                                        <strong>{t("sections.roomSettings.permissions.items.delete.label")}</strong>
                                        ：{t("sections.roomSettings.permissions.items.delete.description")}
                                    </li>
                                </ul>
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title={t("sections.roomSharing.title")} icon={Share2}>
                            <HelpItem title={t("sections.roomSharing.shareLink.title")} icon={Share2}>
                                {t("sections.roomSharing.shareLink.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.roomSharing.qrcode.title")} icon={QrCode}>
                                {t("sections.roomSharing.qrcode.description")}
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title={t("sections.messageOperations.title")} icon={BookCopy}>
                            <HelpItem title={t("sections.messageOperations.editMessage.title")} icon={Pencil}>
                                {t("sections.messageOperations.editMessage.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.messageOperations.copyMessage.title")} icon={Copy}>
                                {t("sections.messageOperations.copyMessage.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.messageOperations.deleteMessage.title")} icon={Trash2}>
                                {t("sections.messageOperations.deleteMessage.description")}
                            </HelpItem>
                            <HelpItem title={t("sections.messageOperations.saveChanges.title")} icon={Save}>
                                {t("sections.messageOperations.saveChanges.description")}
                            </HelpItem>
                        </HelpSection>
                    </div>
                </ScrollArea>
            </DialogContent>
        </Dialog>
    );
}
