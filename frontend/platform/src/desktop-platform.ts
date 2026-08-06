import { invoke } from '@tauri-apps/api/core';

import type { DesktopRuntimeInfo, PlatformAdapter } from './platform-adapter';

/** Tauri Desktop 运行时适配器；本地 Panel 地址和引导凭据由 Rust 容器提供。 */
export const desktopPlatform: PlatformAdapter = {
  kind: 'desktop',
  apiBaseUrl: window.location.origin,
  initialize: () => invoke<DesktopRuntimeInfo>('desktop_runtime'),
  completeInitialAdmin: () => invoke('complete_initial_admin'),
};
