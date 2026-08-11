<script setup lang="ts">
/*
 * AppShell：应用外壳（docs/design/frontend-design.md §3.1）。
 * 桌面端为玻璃拟态侧边导航 + 顶栏，窄屏（<1024px）收起为底部图标标签栏；
 * 页面内容通过默认插槽注入，外壳不感知具体页面。
 * props/emits 与原 ControlPanelHeader 对齐，WorkspaceView 仅做替换。
 */
import { Button as AButton, Tooltip as ATooltip } from '@arco-design/web-vue';
import {
  IconApps,
  IconCloud,
  IconDashboard,
  IconMenuFold,
  IconMenuUnfold,
  IconPoweroff,
  IconRefresh,
  IconSettings,
  IconUser,
} from '@arco-design/web-vue/es/icon';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute } from 'vue-router';

import type { Core, User } from '@mcnp/api-client';

import PreferenceControls from './PreferenceControls.vue';

const props = defineProps<{
  user: User;
  cores: Core[];
  loading: boolean;
  signingOut: boolean;
  showSignOut: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
  signOut: [];
}>();

const { t } = useI18n();
const route = useRoute();
const collapsed = ref(false);

const onlineCoreCount = computed(() => props.cores.filter((core) => core.status === 'ONLINE').length);
const userInitial = computed(() => props.user.displayName.trim().charAt(0).toLocaleUpperCase() || 'U');

const navItems = computed(() => {
  const items = [
    { key: 'dashboard', icon: IconDashboard, label: t('nav.dashboard'), to: { name: 'dashboard' } },
    { key: 'instances', icon: IconApps, label: t('nav.instances'), to: { name: 'instances' } },
    { key: 'nodes', icon: IconCloud, label: t('nav.nodes'), to: { name: 'nodes' } },
    { key: 'settings', icon: IconSettings, label: t('nav.settings'), to: { name: 'settings' } },
  ];
  if (props.user.permissions.includes('user.manage')) {
    items.splice(3, 0, { key: 'users', icon: IconUser, label: t('nav.users'), to: { name: 'users' } });
  }
  return items;
});

/** 实例相关路由（目录、Core 实例列表、实例工作区）共享 instances 高亮 */
function isActive(key: string): boolean {
  if (key === 'instances') {
    return route.path.startsWith('/instances');
  }
  return route.path === `/${key}`;
}

/** 顶栏标题：按当前路由名取 i18n 文案 */
const pageTitle = computed(() => {
  const key = String(route.name ?? '');
  const navKey = {
    'core-instances': 'instances',
    'instance-workspace': 'workspace',
  }[key];
  return t(`nav.${navKey ?? key}`);
});
</script>

<template>
  <div class="app-shell">
    <aside class="app-sidebar mcnp-glass" :class="{ 'app-sidebar--collapsed': collapsed }">
      <RouterLink class="app-brand" :to="{ name: 'dashboard' }" :aria-label="t('app.name')">
        <span class="app-brand__mark" aria-hidden="true">M</span>
        <strong v-if="!collapsed" class="app-brand__name">{{ t('app.shortName') }}</strong>
      </RouterLink>

      <nav class="app-nav" :aria-label="t('nav.controlPanel')">
        <RouterLink
          v-for="item in navItems"
          :key="item.key"
          class="app-nav__item"
          :class="{ 'app-nav__item--active': isActive(item.key) }"
          :to="item.to"
          :title="collapsed ? item.label : undefined"
        >
          <component :is="item.icon" class="app-nav__icon" />
          <span v-if="!collapsed" class="app-nav__label">{{ item.label }}</span>
        </RouterLink>
      </nav>

      <div class="app-sidebar__footer">
        <a-tooltip v-if="collapsed" :content="t('common.collapse')">
          <a-button
            class="app-sidebar__collapse"
            type="text"
            size="mini"
            :aria-label="t('common.collapse')"
            @click="collapsed = !collapsed"
          >
            <template #icon>
              <IconMenuUnfold v-if="collapsed" />
              <IconMenuFold v-else />
            </template>
          </a-button>
        </a-tooltip>
      </div>
    </aside>

    <div class="app-main">
      <header class="app-topbar mcnp-glass">
        <div class="app-topbar__title">
          <h1 class="app-topbar__heading">{{ pageTitle }}</h1>
        </div>

        <div class="app-topbar__tools">
          <span class="core-health">
            <i :class="{ 'core-health__dot--online': onlineCoreCount > 0 }" aria-hidden="true"></i>
            <span class="mcnp-num">{{ onlineCoreCount }}/{{ cores.length }}</span>
            <span class="core-health__label">{{ t('cores.title') }}</span>
          </span>
          <a-tooltip :content="t('common.refresh')">
            <a-button
              class="app-topbar__icon-button"
              type="text"
              size="small"
              :loading="loading"
              :aria-label="t('common.refresh')"
              @click="emit('refresh')"
            >
              <template #icon><IconRefresh /></template>
            </a-button>
          </a-tooltip>
          <PreferenceControls />
          <div class="user-identity">
            <span class="user-avatar">{{ userInitial }}</span>
            <span class="user-name">{{ user.displayName }}</span>
          </div>
          <a-tooltip v-if="showSignOut" :content="t('nav.logout')">
            <a-button
              class="app-topbar__icon-button"
              type="text"
              size="small"
              :loading="signingOut"
              :aria-label="t('nav.logout')"
              @click="emit('signOut')"
            >
              <template #icon><IconPoweroff /></template>
            </a-button>
          </a-tooltip>
        </div>
      </header>

      <main class="app-content">
        <slot />
      </main>
    </div>

    <nav class="app-tabbar mcnp-glass" :aria-label="t('nav.controlPanel')">
      <RouterLink
        v-for="item in navItems"
        :key="item.key"
        class="app-tabbar__item"
        :class="{ 'app-tabbar__item--active': isActive(item.key) }"
        :to="item.to"
        :aria-label="item.label"
      >
        <component :is="item.icon" class="app-tabbar__icon" />
      </RouterLink>
    </nav>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  min-height: 100vh;
  grid-template-columns: 14.5rem minmax(0, 1fr);
}

/* ---- 侧边导航 ---- */

.app-sidebar {
  position: sticky;
  z-index: 30;
  top: 0;
  display: flex;
  height: 100vh;
  flex-direction: column;
  border-right: 1px solid var(--mcnp-border);
  padding: 0.9rem 0.75rem 0.75rem;
  transition: width 180ms ease-out;
}

.app-sidebar--collapsed {
  width: 4rem;
}

.app-brand {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 1.25rem;
  padding: 0.2rem 0.35rem;
  color: var(--mcnp-text);
  text-decoration: none;
}

.app-brand__mark {
  display: grid;
  width: 2.1rem;
  height: 2.1rem;
  flex: 0 0 2.1rem;
  border-radius: 10px;
  place-items: center;
  background: var(--mcnp-gradient-primary);
  color: #fff;
  font-size: 1rem;
  font-weight: 800;
  box-shadow: var(--mcnp-shadow);
}

.app-brand__name {
  font-size: 0.92rem;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.app-nav {
  display: grid;
  gap: 0.25rem;
}

.app-nav__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.7rem;
  border-radius: var(--mcnp-radius-sm);
  padding: 0.55rem 0.75rem;
  color: var(--mcnp-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
  text-decoration: none;
  transition:
    background-color 150ms ease-out,
    color 150ms ease-out;
}

.app-nav__item:hover {
  background: var(--mcnp-surface-hover);
  color: var(--mcnp-text);
}

.app-nav__item--active {
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
}

.app-nav__item--active::before {
  position: absolute;
  left: 0;
  width: 3px;
  height: 1.15rem;
  border-radius: 0 3px 3px 0;
  background: var(--mcnp-gradient-primary);
  content: '';
}

.app-nav__icon {
  flex: 0 0 auto;
  font-size: 1.05rem;
}

.app-nav__label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-sidebar--collapsed .app-nav__item {
  justify-content: center;
  padding: 0.55rem 0;
}

.app-sidebar__footer {
  display: flex;
  justify-content: flex-end;
  margin-top: auto;
}

.app-sidebar__collapse {
  color: var(--mcnp-text-faint);
}

/* ---- 顶栏 ---- */

.app-main {
  display: flex;
  min-width: 0;
  min-height: 100vh;
  flex-direction: column;
}

.app-topbar {
  position: sticky;
  z-index: 20;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 3.5rem;
  gap: 1rem;
  border-bottom: 1px solid var(--mcnp-border);
  padding: 0 1.5rem;
}

.app-topbar__heading {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.95rem;
  font-weight: 650;
}

.app-topbar__tools {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 0.3rem;
}

.core-health {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  margin-right: 0.35rem;
  color: var(--mcnp-text-muted);
  font-size: 0.72rem;
  white-space: nowrap;
}

.core-health i {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: var(--mcnp-text-faint);
}

.core-health i.core-health__dot--online {
  background: var(--mcnp-success);
}

.core-health__label {
  color: var(--mcnp-text-faint);
}

.app-topbar__icon-button {
  width: 2rem;
  height: 2rem;
  color: var(--mcnp-text-muted);
}

.user-identity {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 0.5rem;
  margin-left: 0.35rem;
  padding-left: 0.7rem;
  border-left: 1px solid var(--mcnp-border);
}

.user-avatar {
  display: grid;
  width: 1.75rem;
  height: 1.75rem;
  flex: 0 0 1.75rem;
  border-radius: 50%;
  place-items: center;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
  font-size: 0.72rem;
  font-weight: 700;
}

.user-name {
  max-width: 8rem;
  overflow: hidden;
  color: var(--mcnp-text-muted);
  font-size: 0.76rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-content {
  width: min(100%, 120rem);
  flex: 1;
  margin: 0 auto;
  padding: 1.25rem 1.5rem 2rem;
}

/* ---- 移动端底部标签栏（<1024px 隐藏侧边导航） ---- */

.app-tabbar {
  display: none;
}

@media (max-width: 63.99rem) {
  .app-shell {
    grid-template-columns: minmax(0, 1fr);
  }

  .app-sidebar {
    display: none;
  }

  .app-topbar {
    padding: 0 0.85rem;
  }

  .core-health__label,
  .user-name {
    display: none;
  }

  .app-content {
    padding: 1rem 0.85rem 4.5rem;
  }

  .app-tabbar {
    position: fixed;
    z-index: 40;
    right: 0.75rem;
    bottom: 0.75rem;
    left: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: space-around;
    border: 1px solid var(--mcnp-border);
    border-radius: var(--mcnp-radius-lg);
    padding: 0.4rem 0.5rem;
    box-shadow: var(--mcnp-shadow-hover);
  }

  .app-tabbar__item {
    display: grid;
    flex: 1;
    place-items: center;
    border-radius: var(--mcnp-radius-sm);
    padding: 0.45rem 0;
    color: var(--mcnp-text-faint);
    text-decoration: none;
  }

  .app-tabbar__item--active {
    background: var(--mcnp-primary-soft);
    color: var(--mcnp-primary);
  }

  .app-tabbar__icon {
    font-size: 1.2rem;
  }
}
</style>
