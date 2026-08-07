export type PlatformKind = 'browser' | 'desktop' | 'mobile';

export interface DesktopRuntimeInfo {
  apiBaseUrl: string;
}

/** Desktop 容器代为创建受信任会话后返回给共享前端的最小用户信息。 */
export interface DesktopSessionUser {
  id: string;
  username: string;
  displayName: string;
  permissions: string[];
  resourceScopes?: string[];
}

/** Desktop 受信任会话沿用 Panel 标准登录响应，便于前端复用令牌生命周期。 */
export interface DesktopSessionResponse {
  user: DesktopSessionUser;
  session: {
    id: string;
    accessToken: string | null;
    accessExpiresAt: string;
    refreshToken: string | null;
    refreshExpiresAt: string | null;
    csrfToken: string | null;
  };
}

export interface PlatformAdapter {
  kind: PlatformKind;
  apiBaseUrl: string;
  initialize?: () => Promise<DesktopRuntimeInfo>;
  createDesktopSession?: () => Promise<DesktopSessionResponse>;
  isAutostartEnabled?: () => Promise<boolean>;
  setAutostartEnabled?: (enabled: boolean) => Promise<boolean>;
  openLogDirectory?: () => Promise<void>;
  getRefreshToken?: () => Promise<string | null>;
  setRefreshToken?: (refreshToken: string) => Promise<void>;
  clearRefreshToken?: () => Promise<void>;
}
