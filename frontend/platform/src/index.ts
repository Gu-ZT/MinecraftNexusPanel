/**
 * @mcnp/platform —— MCNP 平台能力适配层。
 *
 * 三端（Browser / Tauri Desktop / Tauri Mobile）共享同一套业务代码，
 * 平台差异只能经本包接口访问（PLAN.md 第 4.2 节）。
 */

export type { AuthStorage, AuthTokenBundle, PlatformAdapter, PlatformKind } from './adapter';
export { createBrowserPlatform } from './browser';

// TODO(M5): createTauriPlatform —— Tauri Desktop/Mobile 适配器，
// 令牌读写走系统安全存储，Access/Refresh Token 绑定设备会话。
