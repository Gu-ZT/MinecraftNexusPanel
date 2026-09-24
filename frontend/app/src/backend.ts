/**
 * 全局单例：后端客户端与平台适配器。
 *
 * TODO(M1): 按 VITE_USE_MOCK 开关切换为真实 HTTP/WebSocket 客户端，
 * api 与 realtime 的接口签名保持不变。
 */

import { createMockBackend } from '@mcnp/api-client';
import { createBrowserPlatform } from '@mcnp/platform';

export const backend = createMockBackend();
export const platform = createBrowserPlatform();

// 启动时凭本地令牌快照恢复 Mock 会话（整页刷新后免重新登录）；
// 真实客户端由 Cookie/Authorization 头自动完成，不需要此步。TODO(M1)
{
  const snapshot = platform.authStorage.read();
  if (snapshot) backend.store.restoreSessionByAccessToken(snapshot.accessToken);
}
