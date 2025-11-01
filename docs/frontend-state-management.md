# Elizabeth 项目前端状态管理详细文档

## 模块概述

状态管理模块是 Elizabeth 项目的核心架构模块，基于 Zustand
状态管理库实现全局状态管理。该模块提供了完整的状态管理解决方案，包括状态结构定义、自定义
Hooks、持久化策略、数据流管理和性能优化。

## 功能描述

状态管理模块主要负责：

- 提供统一的全局状态存储和更新机制
- 管理应用的主题、字体大小、用户偏好等设置
- 管理房间、消息、文件选择等应用状态
- 提供状态持久化和恢复功能
- 支持离线编辑和乐观更新机制
- 集成权限系统和认证状态管理

## 状态结构详细分析

### 应用状态完整结构

#### AppState 接口定义

```typescript
interface AppState {
  // 主题管理
  theme: Theme;
  setTheme: (theme: Theme) => void;
  cycleTheme: () => void;

  // 设置管理
  sendOnEnter: boolean;
  setSendOnEnter: (value: boolean) => void;

  // 字体大小管理
  editorFontSize: number;
  setEditorFontSize: (size: number) => void;
  toolbarButtonSize: number;
  setToolbarButtonSize: (size: number) => void;
  messageFontSize: number;
  setMessageFontSize: (size: number) => void;

  // 文件和消息选择管理
  selectedFiles: Set<string>;
  toggleFileSelection: (fileId: string) => void;
  clearFileSelection: () => void;
  selectedMessages: Set<string>;
  toggleMessageSelection: (messageId: string) => void;
  clearMessageSelection: () => void;

  // 侧边栏状态
  leftSidebarCollapsed: boolean;
  toggleLeftSidebar: () => void;

  // 房间管理
  currentRoomId: string;
  setCurrentRoomId: (roomId: string) => void;

  // 认证状态
  isAuthenticated: (roomName?: string) => boolean;
  getCurrentRoomToken: () => TokenInfo | null;

  // 消息管理
  messages: LocalMessage[];
  setMessages: (messages: Message[]) => void;
  addMessage: (content: string) => void;
  updateMessageContent: (messageId: string, content: string) => void;
  markMessageForDeletion: (messageId: string) => void;
  revertMessageChanges: (messageId: string) => void;
  hasUnsavedChanges: () => boolean;
  saveMessages: () => Promise<void>;

  // UI 偏好设置
  includeMetadataInCopy: boolean;
  includeMetadataInDownload: boolean;
  includeMetadataInExport: boolean;
  showDeleteConfirmation: boolean;

  // heti 支持
  useHeti: boolean;
  setUseHeti: (value: boolean) => void;
}
```

### 状态分片职责分析

#### 主题管理分片

- **状态**: `theme: Theme`
- **操作**: `setTheme`, `cycleTheme`
- **职责**: 管理应用主题切换，支持 dark/light/system 三种模式

#### UI 设置分片

- **状态**: 字体大小、发送行为、侧边栏状态
- **操作**: 相应的 setter 函数
- **职责**: 管理用户界面偏好和交互行为

#### 选择管理分片

- **状态**: `selectedFiles: Set<string>`, `selectedMessages: Set<string>`
- **操作**: 切换、清除、全选、反选等批量操作
- **职责**: 管理文件和消息的多选状态

#### 房间管理分片

- **状态**: `currentRoomId: string`
- **操作**: `setCurrentRoomId`
- **职责**: 跟踪当前活跃房间，切换房间时清空消息状态

#### 认证分片

- **状态**: 衍生状态 `isAuthenticated`, `getCurrentRoomToken`
- **操作**: 基于 localStorage 中的 token 信息
- **职责**: 提供房间访问权限验证

#### 消息管理分片

- **状态**: `messages: LocalMessage[]`
- **操作**: 完整的 CRUD 操作和状态同步
- **职责**: 管理本地消息状态，支持离线编辑和批量保存

## 自定义 Hooks 分析

### use-room-permissions.ts - 权限状态管理

**功能描述**: 提供房间权限检查和管理功能，基于 JWT token 进行权限验证。

**核心特性**:

- **Token 获取**: 从 URL 参数或 localStorage 获取房间 token
- **权限解析**: 使用 `getPermissionsFromToken` 解析 JWT 中的权限位
- **权限检查**: 提供 `can.read`, `can.edit` 等便捷方法
- **响应式更新**: 基于 token 变化自动更新权限状态

**实现逻辑**:

```typescript
export function useRoomPermissions() {
  const params = useParams();
  const roomName = params.roomName as string | undefined;

  const token = useMemo(() => {
    if (!roomName) return null;
    return getRoomTokenString(roomName);
  }, [roomName]);

  const permissions = useMemo(() => {
    if (!token) return [];
    return getPermissionsFromToken(token);
  }, [token]);

  const can = useMemo(() => ({
    read: hasPermission(token, "read"),
    edit: hasPermission(token, "edit"),
    share: hasPermission(token, "share"),
    delete: hasPermission(token, "delete"),
  }), [token]);

  return {
    token,
    permissions,
    can,
    roomName: payload?.room_name ?? roomName ?? null,
    roomId: payload?.room_id ?? null,
  };
}
```

### use-mobile.ts - 响应式状态管理

**功能描述**: 检测移动设备状态并提供响应式更新。

**核心特性**:

- **媒体查询**: 使用 `window.matchMedia` 监听屏幕尺寸变化
- **断点设置**: 768px 作为移动设备分界点
- **自动清理**: 组件卸载时移除事件监听器

**实现逻辑**:

```typescript
const MOBILE_BREAKPOINT = 768;

export function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState<boolean | undefined>(
    undefined,
  );

  React.useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    const onChange = () => {
      setIsMobile(window.innerWidth < MOBILE_BREAKPOINT);
    };
    mql.addEventListener("change", onChange);
    setIsMobile(window.innerWidth < MOBILE_BREAKPOINT);
    return () => mql.removeEventListener("change", onChange);
  }, []);

  return !!isMobile;
}
```

### use-toast.ts - 通知状态管理

**功能描述**: 提供全局 Toast 通知系统，支持队列管理和自动清理。

**核心特性**:

- **队列管理**: 限制最多 1 个活跃 Toast，自动移除旧通知
- **自动清理**: 使用 setTimeout 实现延迟自动移除
- **内存管理**: 使用 Map 跟踪超时器，防止内存泄漏

**实现逻辑**:

```typescript
// 使用 useReducer 模式管理 Toast 状态
const [state, setState] = React.useState<State>(memoryState);

function dispatch(action: Action) {
  memoryState = reducer(memoryState, action);
  listeners.forEach((listener) => {
    listener(memoryState);
  });
}

// 支持多种操作类型
const actionTypes = {
  ADD_TOAST: "ADD_TOAST",
  UPDATE_TOAST: "UPDATE_TOAST",
  DISMISS_TOAST: "DISMISS_TOAST",
  REMOVE_TOAST: "REMOVE_TOAST",
} as const;
```

## 数据流和状态更新机制

### 单向数据流

```
用户操作 → Action → Zustand Store → 组件重渲染
```

### 状态更新流程

1. **直接更新**: 通过 setter 函数直接更新状态
2. **批量更新**: 支持多个状态的同时更新
3. **派生状态**: 认证状态基于 token 动态计算

### 变更通知机制

- **自动订阅**: Zustand 自动处理组件订阅
- **精确更新**: 只有实际变化时触发重渲染
- **批量操作**: 减少状态更新次数，提升性能

## 状态持久化和同步策略

### 本地存储策略

#### 存储架构

```typescript
// Token 存储
interface TokenStorage {
  [roomName: string]: TokenInfo;
}

// 持久化配置
{
  name: "elizabeth-storage",
  storage: createJSONStorage(() => localStorage),
  partialize: (state) => ({
    // 只持久化 UI 偏好设置
    sendOnEnter: state.sendOnEnter,
    includeMetadataInCopy: state.includeMetadataInCopy,
    // ... 其他 UI 设置
  }),
}
```

#### 存储策略特点

- **选择性持久化**: 只持久化 UI 偏好设置，排除临时数据
- **JSON 序列化**: 使用 `createJSONStorage` 处理复杂对象
- **自动清理**: 过期 token 自动清理

### 状态与 API 同步

#### Token 管理

```typescript
// 自动刷新机制
export async function refreshRoomToken(
  roomName: string,
): Promise<string | null> {
  try {
    const response = await fetch(`${API_BASE_URL}/rooms/${roomName}/tokens`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });

    if (data.token) {
      const tokenInfo: TokenInfo = {
        token: data.token,
        expiresAt: data.expires_at,
      };

      setRoomToken(roomName, tokenInfo);
      return data.token;
    }
  } catch (error) {
    console.error("Error refreshing token for room:", roomName, error);
    return null;
  }
}
```

#### 同步策略

- **乐观更新**: 本地立即更新，后台同步服务器
- **冲突解决**: 服务器数据优先，本地数据作为回退
- **错误重试**: 自动重试机制，支持指数退避

### 离线状态处理

#### 离线消息管理

```typescript
interface LocalMessage extends Message {
  isOwn?: boolean;
  isNew?: boolean;
  isDirty?: boolean;
  isPendingDelete?: boolean;
  originalContent?: string;
}
```

**离线特性**:

- **本地编辑**: 支持离线创建和编辑消息
- **状态标记**: `isNew`, `isDirty`, `isPendingDelete` 标记消息状态
- **批量同步**: `saveMessages()` 函数批量处理未保存的更改

## 性能优化和最佳实践

### 状态管理性能优化

#### 选择性订阅

```typescript
// 使用 useMemo 优化昂贵的计算
const permissions = useMemo(() => {
  if (!token) return [];
  return getPermissionsFromToken(token);
}, [token]);
```

#### 批量操作

```typescript
// 文件批量选择优化
selectA;
```
