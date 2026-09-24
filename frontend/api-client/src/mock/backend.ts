/**
 * createMockBackend：组装 MockApi + MockRealtime，共享同一份内存状态。
 */

import type { McnpApi } from '../api';
import type { RealtimeClient } from '../realtime';
import { MockApi } from './api';
import { MockRealtime } from './realtime';
import { MockStore } from './store';

export interface MockBackend {
  api: McnpApi;
  realtime: RealtimeClient;
  store: MockStore;
}

export function createMockBackend(): MockBackend {
  const store = new MockStore();
  return {
    api: new MockApi(store),
    realtime: new MockRealtime(store),
    store,
  };
}
