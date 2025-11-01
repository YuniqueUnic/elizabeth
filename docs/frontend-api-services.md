# Elizabeth 项目前端 API 服务层详细文档

## 模块概述

API 服务层是 Elizabeth 项目的前端核心基础设施模块，负责与后端 Rust
服务器的所有通信交互。该模块采用统一的架构设计，提供了完整的 API
封装、错误处理、认证管理、缓存策略和离线支持。

## 功能描述

API 服务层模块主要负责：

- 提供统一的 HTTP 请求处理框架
- 集成 JWT 令牌管理和自动刷新机制
- 支持房间、消息、文件、权限、分享等核心业务 API
- 提供智能的错误处理和重试机制
- 支持大文件分块上传和进度跟踪
- 提供离线支持和数据同步功能
- 实现完整的缓存策略和性能优化

## 服务清单

### 1. authService.ts - 认证服务

**文件位置**: `web/api/authService.ts`

**功能描述**: JWT 令牌管理、房间访问令牌获取、令牌验证、令牌刷新和用户登出

**核心方法**:

- `getAccessToken(roomName, password?, options?)`: 获取房间访问令牌
- `validateToken(roomName, token?)`: 验证令牌有效性
- `refreshToken(refreshToken)`: 刷新过期令牌
- `revokeRoomToken(roomName, jti, token?)`: 撤销房间令牌
- `getValidToken(roomName)`: 获取有效令牌

**技术特点**:

- JWT 令牌管理基于标准
- 自动刷新机制确保会话连续性
- 本地存储用于令牌持久化
- 安全存储策略（密码不存储在客户端）

### 2. messageService.ts - 消息服务

**文件位置**: `web/api/messageService.ts`

**功能描述**: 聊天消息的发送、接收、删除和更新，支持文本消息和文件附件

**核心方法**:

- `getMessages(roomName, token?)`: 获取消息列表
- `postMessage(roomName, content, token?)`: 发送消息
- `deleteMessage(roomName, messageId, token?)`: 删除消息
- `updateMessage(roomName, messageId, content, token?)`: 更新消息

**技术特点**:

- 两阶段上传机制（预留空间→上传内容）
- 内容类型过滤（文本消息 vs 文件附件）
- FormData 处理文件上传
- 消息按时间戳排序显示

### 3. fileService.ts - 文件服务

**文件位置**: `web/api/fileService.ts`

**功能描述**: 文件的上传、下载、删除和批量操作，支持大文件分块上传

**核心方法**:

- `getFilesList(roomName, token?)`: 获取文件列表
- `uploadFile(roomName, file, token?, options?)`: 文件上传（支持分块）
- `deleteFile(roomName, fileId, token?)`: 删除文件
- `deleteFiles(roomName, fileIds, token?)`: 批量删除文件
- `downloadFile(roomName, fileId, fileName, token?)`: 文件下载
- `downloadFilesBatch(roomName, fileIds, token?)`: 批量下载

**技术特点**:

- 智能分块（文件大于 5MB 自动切换）
- 进度跟踪和实时回调
- 断点续传功能
- 文件类型识别和验证

### 4. roomService.ts - 房间服务

**文件位置**: `web/api/roomService.ts`

**功能描述**: 房间的创建、获取详情、更新设置、权限管理和删除

**核心方法**:

- `createRoom(name, password?)`: 创建房间
- `getRoomDetails(roomName, token?, skipAuth?)`: 获取房间详情
- `deleteRoom(roomName, token?)`: 删除房间
- `updateRoomPermissions(roomName, permissions, token?)`: 更新房间权限
- `updateRoomSettings(roomName, settings, token?)`: 更新房间设置
- `listRoomTokens(roomName, token?)`: 列出房间令牌

**技术特点**:

- RESTful 设计遵循 RESTful API 原则
- 权限位编码使用位运算存储
- 设置灵活支持多种配置选项
- 状态管理统一和同步

### 5. permissionService.ts - 权限服务

**文件位置**: `web/api/permissionService.ts`

**功能描述**: 权限的检查、验证、设置和管理，提供权限预设和工具函数

**核心方法**:

- `setRoomPermissions(roomName, permissions, makePermanent?, token?)`:
  设置房间权限
- `getUserPermissions(roomName, token?)`: 获取用户权限
- `hasPermission(roomName, token?, permission)`: 检查具体权限
- `createPermissionAwareAPI(apiCall, permissionCheck, errorMessage)`:
  创建权限感知 API 包装器

**技术特点**:

- 位编码权限系统高效且灵活
- 权限预设提供常用权限组合
- 工具函数简化权限检查操作
- 严格的 TypeScript 类型定义

### 6. shareService.ts - 分享服务

**文件位置**: `web/api/shareService.ts`

**功能描述**: 房间分享链接生成、QR 码生成、分享令牌创建、Web Share API 集成

**核心方法**:

- `getShareLink(roomName, token?)`: 获取分享链接
- `createShareToken(roomName, options?, token?)`: 创建分享令牌
- `getQRCodeImage(roomName, options?)`: 生成 QR 码图片
- `accessSharedRoom(roomName, shareToken?, password?)`: 访问分享房间

**技术特点**:

- 多渠道分享（链接、QR 码、Web Share）
- QR 码定制支持主题自适应
- 权限集成确保分享安全
- 浏览器兼容性处理

### 7. roomAccessService.ts - 房间访问服务

**文件位置**: `web/api/roomAccessService.ts`

**功能描述**: 房间访问的完整流程管理，包括密码验证、UUID
生成、可用性检查和缓存机制

**核心方法**:

- `checkRoomAvailability(roomName)`: 检查房间可用性
- `generateRoomUUID()`: 生成房间 UUID
- `accessRoomWithPassword(roomName, password)`: 密码保护房间访问
- `accessShareableRoom(roomName)`: 可分享房间访问
- `accessRoom(roomName, options?)`: 完整访问流程

**技术特点**:

- 智能访问方式自动选择
- UUID 生成用于非分享房间
- 缓存机制提升访问效率
- 统一的房间访问状态管理

### 8. chunkedUploadService.ts - 分块上传服务

**文件位置**: `web/api/chunkedUploadService.ts`

**功能描述**: 大文件的分块上传、进度跟踪、断点续传和文件合并功能

**核心方法**:

- `uploadFileChunked(roomName, file, options?, token?)`: 分块上传主函数
- `getChunkedUploadStatus(roomName, uploadId, token?)`: 查询上传状态
- `completeChunkedUpload(roomName, uploadId, token?)`: 完成分块上传

**技术特点**:

- 1MB 默认分块大小，支持自定义配置
- 详细的上传进度信息和速度计算
- 断点续传支持上传中断恢复
- SHA256 文件哈希验证确保完整性

## 实现细节

### 统一 API 处理框架

#### 核心架构

```typescript
// 统一 API 对象
export const api = {
  get: <T = any>(path, params?, options?) =>
    request<T>(path, { ...options, method: "GET" }),

  post: <T = any>(path, data?, options?) =>
    request<T>(path, { ...options, method: "POST", body: data }),

  put: <T = any>(path, data?, options?) =>
    request<T>(path, { ...options, method: "PUT", body: data }),

  delete: <T = any>(path, data?, options?) =>
    request<T>(path, { ...options, method: "DELETE", body: data }),
};
```

#### 请求处理核心

```typescript
async function request<T>(
  path: string,
  options: RequestOptions = {},
): Promise<T> => {
  // URL 构建和参数处理
  const url = buildURL(path, params);

  // 令牌注入逻辑
  let finalUrl = url;
  if (!options.skipTokenInjection && token) {
    finalUrl = injectToken(url, token);
  }

  // 请求配置和重试机制
  const config = { ...REQUEST_CONFIG, ...options };

  // 执行请求和错误处理
  let lastError: Error | null = null;
  let tokenRefreshAttempted = false;

  for (let attempt = 0; attempt <= config.retries; attempt++) {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), REQUEST_CONFIG.timeout);

      const response = await fetch(finalUrl, {
        ...fetchOptions,
        headers,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      // 401 错误处理和令牌刷新
      if (response.status === 401 && roomName && !token && !tokenRefreshAttempted) {
        clearRoomToken(roomName);
        tokenRefreshAttempted = true;

        const freshToken = await refreshRoomToken(roomName);
        if (freshToken) {
          finalUrl = buildURL(path);
          finalUrl = injectToken(finalUrl, freshToken);
          attempt--;
          continue;
        }
      }

      return await parseResponse<T>(response, config.responseType);
    } catch (error) {
      lastError = error as Error;
      // 重试逻辑
      if (attempt < config.retries) {
        await new Promise(resolve => setTimeout(resolve, config.retryDelay * (attempt + 1)));
      }
    }
  }

  throw lastError || new APIError("Request failed after retries");
}
```

### 错误处理机制

#### 统一错误处理

```typescript
export class APIError extends Error {
  constructor(
    message: string,
    public code?: number,
    public response?: Response,
  ) {
    super(message);
    this.name = "APIError";
  }
}
```

#### 重试策略

- **指数退避**: 失败后等待时间指数增长
- **智能重试**: 401 错误时自动尝试令牌刷新
- **超时控制**: 可配置的请求超时时间
- **错误分类**: 区分客户端错误和服务端错误

### 缓存策略和数据同步

#### 多层缓存策略

```typescript
// 内存缓存
const cache = new Map<string, any>();

// localStorage 缓存
const storageCache = {
  get: (key: string) => localStorage.getItem(key),
  set: (key: string, value: any) =>
    localStorage.setItem(key, JSON.stringify(value)),
};

// 智能缓存策略
const cacheStrategy = {
  shouldCache: (key: string, data: any) => {
    const age = getCacheAge(key);
    return age < CACHE_CONFIG.maxAge && data.size < CACHE_CONFIG.maxSize;
  },
  invalidate: (key: string) => storageCache.remove(key),
};
```

#### 同步策略

- **乐观更新**: 本地立即更新，后台同步服务器
- **冲突解决**: 使用版本号和时间戳解决数据冲突
- **增量同步**: 只同步变更的数据减少网络传输

### 离线支持

#### 离线功能

```typescript
export const offlineManager = {
  isOnline: () => navigator.onLine,
  queueOfflineRequests: (request: () => Promise<any>) => void,
  processOfflineQueue: () => Promise<void>,
};
```

#### 离线功能

- **离线队列**: 网络不可用时将请求排队
- **数据同步**: 网络恢复时自动同步离线期间的变更
- **冲突解决**: 离线期间的数据冲突解决机制

## 使用示例

### 认证服务使用示例

```typescript
import { authService } from "@/api/authService";

export default function LoginForm() {
  const handleLogin = async (roomName: string, password: string) => {
    try {
      const token = await authService.getAccessToken(roomName, password);
      if (token) {
        // 保存令牌并跳转到房间
        localStorage.setItem("roomToken", token);
        window.location.href = `/room/${roomName}`;
      }
    } catch (error) {
      console.error("登录失败：", error);
    }
  };

  return (
    <form onSubmit={handleLogin}>
      <input name="roomName" placeholder="房间名称" />
      <input name="password" type="password" placeholder="密码" />
      <button type="submit">登录</button>
    </form>
  );
}
```

### 文件服务使用示例

```typescript
import { fileService } from "@/api/fileService";

export default function FileUpload() {
  const [uploadProgress, setUploadProgress] = React.useState(0);

  const handleFileUpload = async (file: File) => {
    try {
      const result = await fileService.uploadFile("demo-room", file, {
        onProgress: (progress) => setUploadProgress(progress),
      });

      console.log("文件上传成功：", result);
    } catch (error) {
      console.error("上传失败：", error);
    }
  };

  return (
    <div>
      <input
        type="file"
        onChange={(e) => handleFileUpload(e.target.files[0])}
      />
      <div>上传进度：{uploadProgress}%</div>
      <button onClick={() => setUploadProgress(0)}>重置</button>
    </div>
  );
}
```

## 最佳实践

### 开发建议

1. **错误处理**: 实现完善的错误边界和用户反馈机制
2. **类型安全**: 充分利用 TypeScript 的类型检查，确保类型安全
3. **性能优化**: 使用智能缓存策略和请求去重
4. **模块化设计**: 保持服务间的松耦合和高内聚

### 使用注意事项

1. **认证验证**: 所有 API 调用都需要验证用户权限
2. **令牌管理**: 正确处理令牌的存储、刷新和过期
3. **错误重试**: 合理配置重试次数和延迟，避免服务器压力
4. **状态同步**: 确保客户端和服务器状态一致性

### 扩展指南

1. **监控和日志**: 添加 API 调用的监控和日志记录
2. **测试覆盖**: 为 API 服务添加完整的单元测试和集成测试
3. **文档完善**: 补充 API 接口文档和使用示例
4. **版本管理**: 实现 API 版本化的向后兼容性支持

## 技术细节

### 技术栈

- **HTTP 客户端**: 原生 Fetch API，统一请求处理
- **TypeScript**: 完整的类型安全支持
- **状态管理**: Zustand 轻量级状态管理
- **缓存**: localStorage 和内存缓存结合

### 依赖关系

```
前端服务层
├── authService.ts (认证管理)
├── messageService.ts (消息服务)
├── fileService.ts (文件服务)
├── roomService.ts (房间服务)
├── permissionService.ts (权限服务)
├── shareService.ts (分享服务)
├── roomAccessService.ts (房間访问服务)
├── chunkedUploadService.ts (分块上传服务)
└── api.ts (统一请求处理)
```

### 性能特点

- **智能缓存**: 多层缓存策略减少网络请求
- **分块上传**: 大文件的稳定上传解决方案
- **离线支持**: 完整的离线编辑和同步策略
- **请求优化**: 自动重试和错误恢复

## 架构优势

1. **统一架构**: 模块化设计，职责清晰，易于维护
2. **类型安全**: 完整的 TypeScript 类型定义和检查
3. **错误处理**: 健壮的错误处理和用户反馈机制
4. **性能优化**: 智能缓存策略和分块上传优化
5. **用户体验**: 离线支持和进度反馈提升用户体验

该 API
服务层展现了企业级应用的成熟度，在可维护性、可扩展性和用户体验方面都达到了较高的标准。通过统一的接口设计和完善的错误处理机制，为
Elizabeth 项目提供了稳定可靠的前端 API 基础。
