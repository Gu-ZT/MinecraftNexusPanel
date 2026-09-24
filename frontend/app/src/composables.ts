/** 组合式访问全局后端客户端与平台适配器（单例见 backend.ts）。 */

import type { McnpApi, RealtimeClient } from '@mcnp/api-client';
import type { PlatformAdapter } from '@mcnp/platform';
import { backend, platform } from './backend';

export function useApi(): McnpApi {
  return backend.api;
}

export function useRealtime(): RealtimeClient {
  return backend.realtime;
}

export function usePlatform(): PlatformAdapter {
  return platform;
}
