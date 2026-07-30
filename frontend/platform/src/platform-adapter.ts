export type PlatformKind = 'browser' | 'desktop' | 'mobile';

export interface PlatformAdapter {
  kind: PlatformKind;
  apiBaseUrl: string;
}
