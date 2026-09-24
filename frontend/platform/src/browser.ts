import type { AuthStorage, AuthTokenBundle, PlatformAdapter } from './adapter';

const STORAGE_KEY = 'mcnp.mock-auth';

/**
 * 浏览器适配器。
 *
 * 注意：真实浏览器会话由 HttpOnly Cookie 承载（PLAN 4.2），前端脚本不接触会话本体；
 * 这里持久化的仅是 Mock 阶段用于恢复登录态的令牌快照，真实实现落地后本存储只保留
 * 设备 ID 等非敏感字段。TODO(M1): 切换为 Cookie/CSRF 会话。
 */
class BrowserAuthStorage implements AuthStorage {
  read(): AuthTokenBundle | null {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw) as AuthTokenBundle;
      if (parsed.expiresAt <= Date.now()) {
        this.clear();
        return null;
      }
      return parsed;
    } catch (err) {
      console.warn('[platform] 无法解析本地登录快照，已清除', err);
      this.clear();
      return null;
    }
  }

  write(bundle: AuthTokenBundle): void {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(bundle));
  }

  clear(): void {
    window.localStorage.removeItem(STORAGE_KEY);
  }
}

/** 创建浏览器平台适配器。 */
export function createBrowserPlatform(): PlatformAdapter {
  return {
    kind: 'browser',
    authStorage: new BrowserAuthStorage(),
    apiBaseUrl() {
      const override = import.meta.env?.VITE_API_BASE as string | undefined;
      return override ?? `${window.location.origin}/api/v1`;
    },
    wsBaseUrl() {
      const override = import.meta.env?.VITE_WS_BASE as string | undefined;
      if (override) return override;
      const proto = window.location.protocol === 'https:' ? 'wss' : 'ws';
      return `${proto}://${window.location.host}/api/v1/ws`;
    },
    openExternal(url: string) {
      window.open(url, '_blank', 'noopener,noreferrer');
    },
    appInfo() {
      return { name: 'Minecraft Nexus Panel', version: '0.0.0-mock' };
    },
  };
}
