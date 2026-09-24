/**
 * MCNP 平台能力适配层（PLAN.md 第 4.2 节）。
 *
 * 业务代码（app/ui/api-client）不直接访问 window、localStorage、HTTP 地址等
 * 平台细节；一律通过 PlatformAdapter。浏览器与 Tauri 的差异只允许出现在本包。
 */

/** 登录令牌包：Tauri 端使用短期 Access Token + 可轮换 Refresh Token（PLAN 4.2）。 */
export interface AuthTokenBundle {
  accessToken: string;
  refreshToken: string;
  /** 过期时间（epoch 毫秒）。 */
  expiresAt: number;
  /** 令牌绑定的设备会话 ID。 */
  deviceId: string;
}

/** 令牌持久化：浏览器走 Cookie/CSRF 语义，Tauri 走系统安全存储。 */
export interface AuthStorage {
  read(): AuthTokenBundle | null;
  write(bundle: AuthTokenBundle): void;
  clear(): void;
}

export type PlatformKind = 'browser' | 'tauri-desktop' | 'tauri-mobile';

/** 平台能力适配器：每个运行形态恰好注入一个实现。 */
export interface PlatformAdapter {
  readonly kind: PlatformKind;
  readonly authStorage: AuthStorage;
  /** REST 基础地址（含 /api/v1）。 */
  apiBaseUrl(): string;
  /** WebSocket 地址。 */
  wsBaseUrl(): string;
  /** 在系统浏览器中打开外部链接（如模组来源页）。 */
  openExternal(url: string): void;
  appInfo(): { name: string; version: string };
}
