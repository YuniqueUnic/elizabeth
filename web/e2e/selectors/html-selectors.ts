/**
 * HTML 元素选择器映射
 *
 * 此文件维护所有 UI 元素的选择器，采用树形结构对应 UI 层级
 * 支持链式调用：roomPage.roomSettings.password.fill(...)
 */

export const htmlSelectors = {
    // ==================== 顶部导航栏 ====================
    topBar: {
        container: "banner",
        roomInfo: {
            title: 'heading:has-text("Elizabeth")',
            capacity: "text=/房间占用/",
        },
        buttons: {
            copy: 'button:has-text("复制选中消息") >> nth=0',
            download: 'button:has-text("下载导出选中消息") >> nth=0',
            save: 'button:has-text("保存") >> nth=0',
            delete: 'button:has-text("删除") >> nth=0',
            help: 'button:has-text("帮助") >> nth=0',
            settings: 'button:has-text("设置") >> nth=0',
            theme: 'button:has-text("跟随系统") >> nth=0',
        },
    },

    // ==================== 左侧边栏 ====================
    leftSidebar: {
        container: "aside >> nth=0",
        header: {
            title: 'heading:has-text("房间控制")',
            collapseBtn: 'button:has-text("收起侧边栏")',
        },

        // 房间设置部分
        roomSettings: {
            section: 'heading:has-text("房间设置")',

            expirationTime: {
                label: 'text="过期时间"',
                select: "combobox",
                options: {
                    oneMinute: 'option:has-text("1 分钟")',
                    tenMinutes: 'option:has-text("10 分钟")',
                    oneHour: 'option:has-text("1 小时")',
                    twelveHours: 'option:has-text("12 小时")',
                    oneDay: 'option:has-text("1 天")',
                    oneWeek: 'option:has-text("1 周")',
                    never: 'option:has-text("永不过期")',
                },
            },

            password: {
                label: 'text="房间密码"',
                input: 'input[placeholder*="密码"]',
                toggleBtn: "button >> nth=1",
            },

            maxViewCount: {
                label: 'text="最大查看次数"',
                input: "spinbutton",
            },

            saveBtn: 'button:has-text("保存设置")',
        },

        // 房间权限部分
        roomPermissions: {
            section: 'heading:has-text("房间权限")',

            buttons: {
                preview: 'button:has-text("预览")',
                edit: 'button:has-text("编辑")',
                share: 'button:has-text("分享")',
                delete: 'button:has-text("删除")',
            },

            hint: "text=/提示：/",
            saveBtn: 'button:has-text("保存权限")',
        },

        // 分享房间部分
        roomSharing: {
            section: 'heading:has-text("分享房间")',

            qrCode: 'img[alt="Room QR Code"]',

            buttons: {
                getLink: 'button:has-text("获取链接")',
                download: 'button:has-text("下载")',
            },

            roomUrl: "text=/http:/",
        },

        // 容量使用部分
        roomCapacity: {
            section: 'heading:has-text("容量使用")',
            progressBar: "progressbar",
            info: "text=/MB/",
        },
    },

    // ==================== 中间列（消息区域）====================
    middleColumn: {
        container: "main",

        header: {
            messageCount: "text=/共.*条消息/",
            selectAllBtn: 'button:has-text("全选")',
            invertBtn: 'button:has-text("反选")',
        },

        messageList: {
            container: "role=list",
            emptyState: 'text="暂无消息"',

            message: {
                container: '[role="listitem"]',
                content: "paragraph",
                meta: "text=/匿名/",
                timestamp: "text=/刚刚 | 分钟前 | 小时前/",
                unsavedBadge: 'text="未保存"',
                editedBadge: 'text="已编辑"',
                editingBadge: 'text="编辑中"',
                pendingDeleteBadge: 'text="待删除"',
            },
        },

        separator: "separator",

        editor: {
            container: "div >> nth=0",
            toolbar: {
                boldBtn: 'button:has-text("Add bold text")',
                italicBtn: 'button:has-text("Add italic text")',
                strikeBtn: 'button:has-text("Add strikethrough")',
                hrBtn: 'button:has-text("Insert HR")',
                titleBtn: 'button:has-text("Insert title")',
                linkBtn: 'button:has-text("Add a link")',
                quoteBtn: 'button:has-text("Insert a quote")',
                codeBtn: 'button:has-text("Insert code")',
                codeBlockBtn: 'button:has-text("Insert Code Block")',
                commentBtn: 'button:has-text("Insert comment")',
                imageBtn: 'button:has-text("Add image")',
                tableBtn: 'button:has-text("Add table")',
                listBtn: 'button:has-text("Add unordered list")',
                orderedListBtn: 'button:has-text("Add ordered list")',
                checkedListBtn: 'button:has-text("Add checked list")',
                helpBtn: 'button:has-text("Open help")',
            },

            input: 'textarea, [role="textbox"]',

            actions: {
                expandBtn: 'button:has-text("展开编辑器")',
                sendBtn: 'button:has-text("发送")',
            },
        },
    },

    // ==================== 右侧边栏（文件管理）====================
    rightSidebar: {
        container: "complementary >> nth=1",

        header: {
            title: 'heading:has-text("文件管理")',
            uploadBtn: 'button:has-text("上传文件")',
        },

        fileManager: {
            info: "text=/共.*个文件/",

            buttons: {
                selectAllBtn: "button >> nth=0",
                invertBtn: "button >> nth=1",
            },

            fileList: {
                container: '[role="list"]',
                emptyState: 'text="暂无文件"',

                fileItem: {
                    container: '[role="listitem"]',
                    checkbox: "checkbox",
                    name: "text=/.*/",
                    size: "text=/B|KB|MB/",
                    actions: {
                        download: "button >> nth=0",
                        delete: "button >> nth=1",
                    },
                },
            },

            uploadZone: {
                container: '[cursor="pointer"]',
                input: 'input[type="file"]',
                icon: "img",
                text: 'text="拖拽文件到此处或点击上传"',
            },
        },
    },

    // ==================== 对话框与模态框 ====================
    dialogs: {
        deleteConfirmation: {
            container: "role=dialog",
            title: "heading",
            description: "text=/确定要删除/",
            confirmBtn: 'button:has-text("确定")',
            cancelBtn: 'button:has-text("取消")',
            dontAskAgain: 'checkbox:has-text("不再提示")',
        },

        roomSettings: {
            container: "role=dialog",
            title: 'heading:has-text("房间设置")',
            closeBtn: 'button[aria-label="Close"]',
        },

        permissions: {
            container: "role=dialog",
            title: 'heading:has-text("权限设置")',
            closeBtn: 'button[aria-label="Close"]',
        },
    },

    // ==================== 通知与提示 ====================
    notifications: {
        container: 'role=region:has-text("Notifications")',
        toast: {
            container: "role=status",
            title: "text=/成功 | 失败 | 错误/",
            description: "text=/.*/",
            closeBtn: "button",
        },
    },

    // ==================== 首页 ====================
    homepage: {
        logo: 'heading:has-text("Elizabeth")',
        subtitle: "text=/安全、临时、可控/",

        actions: {
            createRoom: 'div:has-text("创建房间")',
            joinRoom: 'div:has-text("加入房间")',
        },

        modal: {
            container: "role=dialog",

            createRoom: {
                title: 'heading:has-text("创建房间")',
                subtitle: "text=/设置房间名称/",

                nameInput: 'textbox:has-text("房间名称")',
                passwordInput: 'textbox:has-text("密码")',

                backBtn: 'button:has-text("返回")',
                createBtn: 'button:has-text("创建房间")',
            },

            joinRoom: {
                title: 'heading:has-text("加入房间")',
                subtitle: "text=/输入房间名称/",

                nameInput: 'textbox:has-text("房间名称")',
                passwordInput: 'textbox:has-text("密码")',

                backBtn: 'button:has-text("返回")',
                joinBtn: 'button:has-text("加入房间")',
            },
        },
    },
};

/**
 * 嵌套选择器获取器
 * 使用方式：getSelector(htmlSelectors.leftSidebar.roomSettings.password.input)
 */
export function getSelector(
    path: string | object,
    defaultSelector: string = "",
): string {
    if (typeof path === "string") {
        return path;
    }

    if (path && typeof path === "object" && "input" in path) {
        return (path as any).input;
    }

    return defaultSelector;
}

/**
 * 选择器验证函数
 * 验证选择器是否有效
 */
export function validateSelector(selector: string): boolean {
    if (!selector || typeof selector !== "string") {
        return false;
    }

    // 检查选择器是否至少包含一个有效的模式
    const validPatterns = [
        /text=/, // Playwright text selector
        /role=/, // ARIA role selector
        /has-text\(/, // Has text selector
        /\[/, // Attribute selector
        /\./, // Class selector
        /^button/, // Tag selector
        /^input/,
        /^textarea/,
    ];

    return validPatterns.some((pattern) => pattern.test(selector));
}

export default htmlSelectors;
