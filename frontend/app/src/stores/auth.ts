/** 鉴权 store：会话、当前用户与权限点集合。 */

import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import type { LoginResult, Permission, User } from '@mcnp/api-client';
import type { PlatformAdapter } from '@mcnp/platform';

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null);
  const permissions = ref<readonly Permission[]>([]);

  const isLoggedIn = computed(() => user.value !== null);

  function has(permission: Permission): boolean {
    return permissions.value.includes(permission);
  }

  function hasAll(required: Permission[]): boolean {
    return required.every((p) => permissions.value.includes(p));
  }

  /** 登录成功后写入会话；令牌经 platform.authStorage 持久化。 */
  function applyLogin(result: LoginResult, platform: PlatformAdapter): void {
    user.value = result.user;
    permissions.value = result.permissions;
    platform.authStorage.write(result.token);
  }

  /** 会话恢复（me 成功）后刷新内存态。 */
  function applySession(session: { user: User; permissions: Permission[] }): void {
    user.value = session.user;
    permissions.value = session.permissions;
  }

  function clear(platform: PlatformAdapter): void {
    user.value = null;
    permissions.value = [];
    platform.authStorage.clear();
  }

  return { user, permissions, isLoggedIn, has, hasAll, applyLogin, applySession, clear };
});
