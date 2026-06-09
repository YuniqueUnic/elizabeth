/**
 * Mutation 工具函数
 * 提供通用的 mutation 操作辅助函数
 */

import { UseToastReturnType } from "@/hooks/use-toast";

interface MutationErrorConfig {
    title?: string;
    description?: string;
}

interface MutationSuccessConfig {
    title?: string;
    description?: string;
    showNotification?: boolean;
}

function numericStatus(value: unknown): number | undefined {
    if (typeof value === "number") return value;
    if (typeof value === "string" && Number.isFinite(Number(value))) {
        return Number(value);
    }
    return undefined;
}

/**
 * Backend permission failures currently arrive either as code
 * `PERMISSION_DENIED` or as a 403 with a permission-related message.
 */
export function isPermissionDeniedError(error: unknown): boolean {
    if (!error || typeof error !== "object") return false;

    const err = error as Record<string, any>;
    const code = err.code ?? err.status ?? err.response?.status;
    if (code === "PERMISSION_DENIED") return true;

    const status = numericStatus(code);
    if (status !== 403) return false;

    const message = typeof err.message === "string" ? err.message : "";
    return /permission|forbidden|denied|权限|无权|没有权限/i.test(message);
}

/**
 * 标准错误处理函数
 */
export const handleMutationError = (
    error: unknown,
    toast: UseToastReturnType["toast"],
    config: MutationErrorConfig = {},
) => {
    let description = config.description;

    if (!description && error && typeof error === "object") {
        const errObj = error as Record<string, any>;
        description = errObj.message;
    }

    toast({
        title: config.title || "操作失败",
        description: description || "请重试",
        variant: "destructive",
    });
};

/**
 * 标准成功处理函数
 */
export const handleMutationSuccess = (
    toast: UseToastReturnType["toast"],
    config: MutationSuccessConfig = {},
) => {
    if (config.showNotification !== false) {
        toast({
            title: config.title || "操作成功",
            description: config.description,
        });
    }
};
