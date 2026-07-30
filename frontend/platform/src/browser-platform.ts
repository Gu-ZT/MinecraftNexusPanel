import type { PlatformAdapter } from './platform-adapter';

export const browserPlatform: PlatformAdapter = {
  kind: 'browser',
  apiBaseUrl: window.location.origin,
};
