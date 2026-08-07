import { invoke } from '@tauri-apps/api/core';

import type {
  DesktopRuntimeInfo,
  DesktopSessionResponse,
  PlatformAdapter,
} from './platform-adapter';

/** Tauri Desktop 运行时适配器；本地 Panel 地址和受信任会话由 Rust 容器提供。 */
export const desktopPlatform: PlatformAdapter = {
  kind: 'desktop',
  apiBaseUrl: window.location.origin,
  initialize: () => invoke<DesktopRuntimeInfo>('desktop_runtime'),
  createDesktopSession: () => invoke<DesktopSessionResponse>('desktop_session'),
  isAutostartEnabled: () => invoke<boolean>('desktop_autostart_enabled'),
  setAutostartEnabled: (enabled) => invoke<boolean>('set_desktop_autostart_enabled', { enabled }),
  openLogDirectory: () => invoke('open_desktop_log_directory'),
  getRefreshToken: () => invoke<string | null>('get_desktop_refresh_token'),
  setRefreshToken: (refreshToken) => invoke('set_desktop_refresh_token', { refreshToken }),
  clearRefreshToken: () => invoke('clear_desktop_refresh_token'),
};
