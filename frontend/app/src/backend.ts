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
