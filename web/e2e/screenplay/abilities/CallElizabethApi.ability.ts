import type { APIRequestContext } from "@playwright/test";
import { Ability } from "@serenity-js/core";

import {
  API_BASE_URL,
  APP_BASE_URL,
  type RoomTokenInfo,
} from "../support/constants";

export interface IssueTokenOptions {
  password?: string;
  role?: string;
  withRefreshToken?: boolean;
  asAdminBootstrap?: boolean;
  /** 身份码有效时长（秒）；仅对非默认角色生效 */
  expiresInSecs?: number;
}

export interface CapabilityGrant {
  capability: string;
  scope: "any" | "own";
}

/** 平台引导凭证（后端 ELIZABETH_ADMIN_TOKEN；仅测试服务器启用）。 */
export const ADMIN_BOOTSTRAP_TOKEN =
  process.env.ELIZABETH_ADMIN_TOKEN ?? "elizabeth-test-admin";

export class CallElizabethApi extends Ability {
  constructor(
    readonly request: APIRequestContext,
    readonly apiBaseUrl: string,
    readonly appBaseUrl: string,
  ) {
    super();
  }

  static using(
    request: APIRequestContext,
    apiBaseUrl = API_BASE_URL,
    appBaseUrl = APP_BASE_URL,
  ): CallElizabethApi {
    return new CallElizabethApi(request, apiBaseUrl, appBaseUrl);
  }

  /**
   * 确保房间存在并返回创建者 admin 身份码。
   * 新建时后端返回创建者 admin token；已存在时用平台引导凭证补签 admin token。
   */
  async ensureRoom(roomName: string, password?: string): Promise<RoomTokenInfo> {
    const createResponse = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}`,
      {
        data: password ? { password } : {},
        timeout: 15_000,
      },
    );

    if (createResponse.ok()) {
      const created = await createResponse.json();
      if (created?.token) {
        return {
          token: created.token as string,
          expiresAt: created.expires_at as string,
          refreshToken: created.refresh_token as string | undefined,
          capabilities: created.capabilities,
          roleKey: created.claims?.role as string | undefined,
        };
      }
    } else if (createResponse.status() !== 409) {
      const body = await createResponse.text().catch(() => "");
      throw new Error(
        `Failed to create room ${roomName}: ${createResponse.status()} ${body}`,
      );
    }

    return this.issueToken(roomName, {
      password,
      role: "admin",
      asAdminBootstrap: true,
    });
  }

  async issueToken(
    roomName: string,
    options: IssueTokenOptions = {},
  ): Promise<RoomTokenInfo> {
    const headers: Record<string, string> = {};
    if (options.asAdminBootstrap) {
      headers["X-Elizabeth-Admin-Token"] = ADMIN_BOOTSTRAP_TOKEN;
    }

    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        data: {
          password: options.password,
          role: options.role,
          with_refresh_token: options.withRefreshToken ?? true,
          expires_in_secs: options.expiresInSecs,
        },
        headers,
        timeout: 15_000,
      },
    );

    if (!response.ok()) {
      const body = await response.text().catch(() => "");
      throw new Error(
        `Failed to issue token for ${roomName}: ${response.status()} ${body}`,
      );
    }

    const token = await response.json();
    return {
      token: token.token as string,
      expiresAt: token.expires_at as string,
      refreshToken: token.refresh_token as string | undefined,
      capabilities: token.capabilities,
      roleKey: token.claims?.role as string | undefined,
    };
  }

  /** 以 admin 身份为他人签发指定角色的身份码。 */
  async issueRoleToken(
    roomName: string,
    role: string,
    adminToken: string,
    expiresInSecs?: number,
  ): Promise<RoomTokenInfo> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        data: {
          token: adminToken,
          role,
          with_refresh_token: true,
          expires_in_secs: expiresInSecs,
        },
        headers: { Authorization: `Bearer ${adminToken}` },
        timeout: 15_000,
      },
    );
    if (!response.ok()) {
      const body = await response.text().catch(() => "");
      throw new Error(
        `Failed to issue ${role} token for ${roomName}: ${response.status()} ${body}`,
      );
    }
    const token = await response.json();
    return {
      token: token.token as string,
      expiresAt: token.expires_at as string,
      refreshToken: token.refresh_token as string | undefined,
      capabilities: token.capabilities,
      roleKey: token.claims?.role as string | undefined,
    };
  }

  /** 签发 editor 身份码但返回原始状态码（席位满 → 409）。 */
  async tryIssueRoleToken(
    roomName: string,
    role: string,
    adminToken: string,
    expiresInSecs?: number,
  ): Promise<number> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        data: {
          token: adminToken,
          role,
          with_refresh_token: false,
          expires_in_secs: expiresInSecs,
        },
        headers: { Authorization: `Bearer ${adminToken}` },
        timeout: 15_000,
      },
    );
    return response.status();
  }

  async roomExists(roomName: string): Promise<boolean> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        data: { with_refresh_token: false },
        timeout: 15_000,
      },
    );

    return response.status() !== 404;
  }

  /** 更新角色能力矩阵（PUT /roles/{key}）。 */
  async updateRole(
    roomName: string,
    roleKey: string,
    capabilities: CapabilityGrant[],
    displayName = roleKey,
    adminToken?: string,
  ): Promise<number> {
    const response = await this.request.put(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/roles/${encodeURIComponent(roleKey)}`,
      {
        data: { display_name: displayName, capabilities },
        headers: adminToken ? { Authorization: `Bearer ${adminToken}` } : {},
        timeout: 15_000,
      },
    );
    return response.status();
  }

  async listRoles(
    roomName: string,
    adminToken?: string,
  ): Promise<Array<{ role_key: string; capabilities: CapabilityGrant[] }>> {
    const response = await this.request.get(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/roles`,
      {
        headers: adminToken ? { Authorization: `Bearer ${adminToken}` } : {},
        timeout: 15_000,
      },
    );
    if (!response.ok()) return [];
    return (await response.json()) as Array<{
      role_key: string;
      capabilities: CapabilityGrant[];
    }>;
  }

  /** 发消息（直接 API，用于准备数据或越权尝试）。 */
  async sendMessage(
    roomName: string,
    text: string,
    token: string,
  ): Promise<{ status: number; id?: number }> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/messages`,
      {
        data: { text },
        headers: { Authorization: `Bearer ${token}` },
        timeout: 15_000,
      },
    );
    const body = await response.json().catch(() => null);
    return { status: response.status(), id: body?.message?.id };
  }

  async listMessages(
    roomName: string,
    token: string,
  ): Promise<{ status: number; items: Array<Record<string, unknown>> }> {
    const response = await this.request.get(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/messages?limit=50`,
      {
        headers: { Authorization: `Bearer ${token}` },
        timeout: 15_000,
      },
    );
    const body = await response.json().catch(() => ({ items: [] }));
    return {
      status: response.status(),
      items: (body?.items ?? []) as Array<Record<string, unknown>>,
    };
  }

  /** 创建 URL 内容（file 域，免 multipart 预留流程）。 */
  async createUrlContent(
    roomName: string,
    data: { url: string; name: string },
    token: string,
  ): Promise<{ status: number; id?: number }> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/contents/url`,
      {
        data,
        headers: { Authorization: `Bearer ${token}` },
        timeout: 15_000,
      },
    );
    const body = await response.json().catch(() => null);
    return { status: response.status(), id: body?.created?.id };
  }

  async listContents(
    roomName: string,
    token: string,
  ): Promise<{ status: number; items: Array<Record<string, unknown>> }> {
    const response = await this.request.get(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/contents`,
      {
        headers: { Authorization: `Bearer ${token}` },
        timeout: 15_000,
      },
    );
    const body = await response.json().catch(() => ({ items: [] }));
    return {
      status: response.status(),
      items: (body ?? []) as Array<Record<string, unknown>>,
    };
  }

  /** 设置内容显隐；返回状态码供越权断言。 */
  async setVisibility(
    roomName: string,
    contentId: number,
    hidden: boolean,
    token: string,
  ): Promise<number> {
    const response = await this.request.put(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/contents/${contentId}/visibility`,
      {
        data: { hidden },
        headers: { Authorization: `Bearer ${token}` },
        timeout: 15_000,
      },
    );
    return response.status();
  }

  /** 直接下载内容（绕过列表的直链路径）。 */
  async downloadStatus(
    contentId: number,
    token: string,
  ): Promise<number> {
    const response = await this.request.get(
      `${this.apiBaseUrl}/contents/${contentId}`,
      {
        params: { token },
        timeout: 15_000,
      },
    );
    return response.status();
  }

  /** 吊销会话（admin 令其它会话立即失效）。 */
  async revokeToken(
    roomName: string,
    jti: string,
    adminToken: string,
  ): Promise<number> {
    const response = await this.request.delete(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens/${encodeURIComponent(jti)}`,
      {
        headers: { Authorization: `Bearer ${adminToken}` },
        timeout: 15_000,
      },
    );
    return response.status();
  }

  async listTokens(
    roomName: string,
    adminToken: string,
  ): Promise<Array<{ jti: string; role_key?: string; revoked_at: string | null }>> {
    const response = await this.request.get(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        headers: { Authorization: `Bearer ${adminToken}` },
        timeout: 15_000,
      },
    );
    if (!response.ok()) return [];
    return (await response.json()) as Array<{
      jti: string;
      role_key?: string;
      revoked_at: string | null;
    }>;
  }
}
