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
    return (
        <Dialog>
            <DialogTrigger asChild>{children}</DialogTrigger>
            <DialogContent className="sm:max-w-3xl max-h-[90vh] flex flex-col overflow-hidden">
                <DialogHeader className="shrink-0">
                    <DialogTitle>帮助文档</DialogTitle>
                    <DialogDescription>
                        Elizabeth 使用指南和功能说明
                    </DialogDescription>
                </DialogHeader>

                <ScrollArea className="flex-1 pr-4 -mr-4 overflow-y-auto">
                    <div className="space-y-8 py-4 pr-4">
                        <HelpSection title="顶部工具栏" icon={Settings}>
                            <HelpItem title="复制按钮" icon={Copy}>
                                复制选中的消息到剪贴板。可以在设置中配置是否包含元信息（时间戳和消息编号）。
                            </HelpItem>
                            <HelpItem title="下载按钮" icon={Download}>
                                将选中的消息导出为 Markdown
                                文件下载到本地。可以在设置中配置是否包含元信息。
                            </HelpItem>
                            <HelpItem title="保存按钮" icon={Save}>
                                保存所有未保存的更改到服务器。当有未保存的消息（新建、编辑或删除）时，按钮会高亮显示。
                            </HelpItem>
                            <HelpItem title="删除按钮" icon={Trash2}>
                                标记选中的消息为待删除状态。需要点击保存按钮后才会真正删除。可以在设置中配置是否显示删除确认对话框。
                            </HelpItem>
                            <HelpItem title="帮助按钮" icon={HelpCircle}>
                                打开帮助文档（当前页面）。
                            </HelpItem>
                            <HelpItem title="设置按钮" icon={Settings}>
                                打开系统设置对话框，配置应用程序的行为和偏好设置。
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title="房间设置" icon={Settings}>
                            <HelpItem title="过期时间" icon={Clock}>
                                设置房间的过期时间。过期后房间内容会被清空并重置。可以选择：1
                                分钟、10 分钟、1 小时、12 小时、1 天、1
                                周或永不过期。
                            </HelpItem>
                            <HelpItem title="房间密码" icon={Lock}>
                                为房间设置密码保护。启用后，访问房间时需要输入密码。
                            </HelpItem>
                            <HelpItem title="最大查看次数" icon={Eye}>
                                设置房间的最大访问次数。达到次数后，房间内容会被清空并重置。
                            </HelpItem>
                            <HelpItem title="房间权限" icon={KeyRound}>
                                设置当前用户对房间的权限：
                                <ul className="list-disc list-inside mt-2 space-y-1">
                                    <li>
                                        <strong>预览</strong>：查看房间内容
                                    </li>
                                    <li>
                                        <strong>编辑</strong>
                                        ：上传和修改内容（需要预览权限）
                                    </li>
                                    <li>
                                        <strong>分享</strong>
                                        ：公开分享房间（需要预览权限）
                                    </li>
                                    <li>
                                        <strong>删除</strong>
                                        ：删除房间内容（需要预览和编辑权限）
                                    </li>
                                </ul>
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title="房间分享" icon={Share2}>
                            <HelpItem title="分享链接" icon={Share2}>
                                当房间有分享权限时，可以通过房间名称直接访问（例如：http://domain.com/room_name）。
                                如果没有分享权限，会生成一个包含 UUID
                                的唯一链接（例如：http://domain.com/room_name_uuid）。
                            </HelpItem>
                            <HelpItem title="二维码" icon={QrCode}>
                                点击"获取链接"按钮可以查看房间的分享链接和二维码。可以下载二维码图片或复制链接分享给其他人。
                            </HelpItem>
                        </HelpSection>

                        <HelpSection title="消息操作" icon={BookCopy}>
                            <HelpItem title="编辑消息" icon={Pencil}>
                                点击消息气泡上的编辑按钮可以编辑消息。编辑时，消息会显示"正在编辑"标识，编辑器顶部会显示编辑提示。
                                编辑完成后点击发送按钮，消息会标记为"已编辑"状态，需要点击保存按钮保存到服务器。
                            </HelpItem>
                            <HelpItem title="复制消息" icon={Copy}>
                                点击消息气泡上的复制按钮可以复制单条消息。复制内容会根据设置中的"复制时包含元信息"配置决定是否包含时间戳和消息编号。
                            </HelpItem>
                            <HelpItem title="删除消息" icon={Trash2}>
                                点击消息气泡上的删除按钮可以标记消息为待删除状态。删除的消息会显示删除线效果。
                                需要点击顶部工具栏的保存按钮后才会真正删除。点击撤销按钮可以取消删除。
                            </HelpItem>
                            <HelpItem title="保存更改" icon={Save}>
                                所有的消息操作（新建、编辑、删除）都需要点击顶部工具栏的保存按钮才会保存到服务器。
                                有未保存的更改时，保存按钮会高亮显示。
                            </HelpItem>
                        </HelpSection>
                    </div>
                </ScrollArea>
            </DialogContent>
        </Dialog>
    );
}
