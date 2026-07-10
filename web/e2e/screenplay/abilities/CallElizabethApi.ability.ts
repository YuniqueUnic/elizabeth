import type { APIRequestContext } from "@playwright/test";
import { Ability } from "@serenity-js/core";

import {
  API_BASE_URL,
  APP_BASE_URL,
  type RoomTokenInfo,
} from "../support/constants";

export interface IssueTokenOptions {
  password?: string;
  withRefreshToken?: boolean;
}

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

  async ensureRoom(roomName: string, password?: string): Promise<void> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}`,
      {
        data: password ? { password } : {},
        timeout: 15_000,
      },
    );

    if (!response.ok() && response.status() !== 409) {
      throw new Error(
        `Failed to create room ${roomName}: ${response.status()} ${response.statusText()}`,
      );
    }
  }

  async issueToken(
    roomName: string,
    options: IssueTokenOptions = {},
  ): Promise<RoomTokenInfo> {
    const response = await this.request.post(
      `${this.apiBaseUrl}/rooms/${encodeURIComponent(roomName)}/tokens`,
      {
        data: {
          password: options.password,
          with_refresh_token: options.withRefreshToken ?? true,
        },
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
    };
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
}
