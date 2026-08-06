export type PlatformKind = 'browser' | 'desktop' | 'mobile';

export interface DesktopRuntimeInfo {
  apiBaseUrl: string;
  initialAdminUsername: string;
  initialAdminPassword: string | null;
}

export interface PlatformAdapter {
  kind: PlatformKind;
  apiBaseUrl: string;
  initialize?: () => Promise<DesktopRuntimeInfo>;
  completeInitialAdmin?: () => Promise<void>;
}
