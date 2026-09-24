<script setup lang="ts">
/** 主界面外壳：侧边导航、Core 切换器、任务指示与用户菜单。 */

import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import type { Permission } from '@mcnp/api-client';
import { useApi, usePlatform } from '@/composables';
import { useAuthStore } from '@/stores/auth';
import { useCoreStore } from '@/stores/core';

interface NavItem {
  key: string;
  label: string;
  permission?: Permission[];
}

const NAV_ITEMS: NavItem[] = [
  { key: '/', label: '总览' },
  { key: '/cores', label: 'Core 节点', permission: ['core.read'] },
  { key: '/instances', label: '实例', permission: ['instance.read'] },
  { key: '/environments', label: '环境管理', permission: ['environment.read'] },
  { key: '/images', label: '镜像管理', permission: ['image.read'] },
  { key: '/tasks', label: '任务中心' },
  { key: '/users', label: '用户', permission: ['user.read'] },
  { key: '/users/groups', label: '用户组', permission: ['user.read'] },
  { key: '/audit', label: '审计日志', permission: ['audit.read'] },
];

const api = useApi();
const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const coreStore = useCoreStore();
const queryClient = useQueryClient();

const visibleNav = computed(() => NAV_ITEMS.filter((item) => !item.permission || auth.hasAll(item.permission)));

const selectedMenuKey = computed(() => {
  const path = route.path;
  if (path === '/') return '/';
  const match = [...visibleNav.value].sort((a, b) => b.key.length - a.key.length).find((item) => path.startsWith(item.key));
  return match?.key ?? path;
});

const { data: cores } = useQuery({
  queryKey: ['cores'],
  queryFn: () => api.cores.list(),
  enabled: computed(() => auth.has('core.read')),
});

const { data: tasks } = useQuery({
  queryKey: ['tasks'],
  queryFn: () => api.tasks.list(),
  refetchInterval: 3_000,
});

const runningTaskCount = computed(() => (tasks.value ?? []).filter((t) => t.status === 'RUNNING' || t.status === 'PENDING').length);

function onMenuClick(key: string): void {
  void router.push(key);
}

function onCoreChange(value: string | null): void {
  coreStore.select(value);
  // 节点级数据源（实例/环境/镜像）随切换失效
  void queryClient.invalidateQueries({ queryKey: ['instances'] });
  void queryClient.invalidateQueries({ queryKey: ['runtimes'] });
  void queryClient.invalidateQueries({ queryKey: ['images'] });
}

async function onUserAction(key: string): Promise<void> {
  if (key === 'settings') {
    void router.push('/settings');
    return;
  }
  if (key === 'logout') {
    await api.auth.logout();
    auth.clear(usePlatform());
    Message.success('已退出登录');
    void router.push('/login');
  }
}
</script>

<template>
  <ALayout class="app-shell">
    <ALayoutSider :width="232" class="app-shell__sider">
      <div class="app-shell__brand">
        <span class="app-shell__logo">⬡</span>
        <span class="app-shell__name">MCNP</span>
      </div>
      <AMenu :selected-keys="[selectedMenuKey]" @menu-item-click="onMenuClick">
        <AMenuItem v-for="item in visibleNav" :key="item.key">{{ item.label }}</AMenuItem>
      </AMenu>
    </ALayoutSider>

    <ALayout>
      <ALayoutHeader class="app-shell__header">
        <div class="app-shell__header-left">
          <ASelect
            v-if="auth.has('core.read')"
            :model-value="coreStore.selectedCoreId ?? '*'"
            class="app-shell__core-select"
            @change="(v) => onCoreChange(v === '*' ? null : String(v))"
          >
            <AOption value="*">全部节点</AOption>
            <AOption v-for="core in cores ?? []" :key="core.id" :value="core.id">
              {{ core.name }}
            </AOption>
          </ASelect>
        </div>
        <div class="app-shell__header-right">
          <ABadge :count="runningTaskCount" :max-count="99">
            <AButton type="text" @click="router.push('/tasks')">任务</AButton>
          </ABadge>
          <ADropdown trigger="click" @select="(v) => onUserAction(String(v))">
            <AButton type="text">{{ auth.user?.displayName ?? '—' }}</AButton>
            <template #content>
              <ADoption value="settings">个人设置</ADoption>
              <ADoption value="logout">退出登录</ADoption>
            </template>
          </ADropdown>
        </div>
      </ALayoutHeader>

      <ALayoutContent class="app-shell__content">
        <!-- key 确保路由参数变化（如切换实例）时重建页面，实时订阅随之重建 -->
        <RouterView :key="route.fullPath" />
      </ALayoutContent>
    </ALayout>
  </ALayout>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
}

.app-shell__sider {
  background: var(--mcnp-bg-panel);
  border-right: 1px solid rgb(255 255 255 / 6%);
}

.app-shell__brand {
  display: flex;
  align-items: center;
  gap: var(--mcnp-space-2);
  height: var(--mcnp-header-height);
  padding: 0 var(--mcnp-space-4);
  font-size: 16px;
  font-weight: 600;
}

.app-shell__logo {
  color: var(--mcnp-color-primary);
}

.app-shell__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--mcnp-header-height);
  padding: 0 var(--mcnp-space-4);
  background: var(--mcnp-bg-panel);
  border-bottom: 1px solid rgb(255 255 255 / 6%);
}

.app-shell__header-left,
.app-shell__header-right {
  display: flex;
  align-items: center;
  gap: var(--mcnp-space-3);
}

.app-shell__core-select {
  width: 200px;
}

.app-shell__content {
  padding: var(--mcnp-space-4) var(--mcnp-space-6);
}
</style>
