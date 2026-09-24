/**
 * 权限上下文：app 层在登录后 provide 当前用户权限点集合，
 * PermissionGate 与页面指令据此渲染（PLAN 4.4：仅 UI 配合，服务端仍强制过滤）。
 */

import type { InjectionKey, Ref } from 'vue';
import type { Permission } from '@mcnp/api-client';

export interface PermissionContext {
  permissions: Ref<readonly Permission[]>;
  has(permission: Permission): boolean;
  hasAll(required: Permission[]): boolean;
}

export const PermissionKey: InjectionKey<PermissionContext> = Symbol('mcnp-permissions');
