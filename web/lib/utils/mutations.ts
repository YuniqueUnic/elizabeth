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

/**
 * 标准错误处理函数
 */
export const handleMutationError = (
  error: unknown,
  toast: UseToastReturnType["toast"],
  config: MutationErrorConfig = {},
) => {
  toast({
    title: config.title || "操作失败",
    description: config.description || "请重试",
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
