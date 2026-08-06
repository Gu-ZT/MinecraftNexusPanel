<script setup lang="ts">
import { Button as AButton, Tooltip as ATooltip } from '@arco-design/web-vue';
import {
  IconApps,
  IconCloud,
  IconDashboard,
  IconPoweroff,
  IconRefresh,
  IconSettings,
} from '@arco-design/web-vue/es/icon';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute } from 'vue-router';

import type { Core, User } from '@mcnp/api-client';

import PreferenceControls from './PreferenceControls.vue';

const props = defineProps<{
  user: User;
  cores: Core[];
  loading: boolean;
  signingOut: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
  signOut: [];
}>();

const { t } = useI18n();
const route = useRoute();
const onlineCoreCount = computed(() => props.cores.filter((core) => core.status === 'ONLINE').length);
const userInitial = computed(() => props.user.displayName.trim().charAt(0).toLocaleUpperCase() || 'U');

function isActive(section: string): boolean {
  if (section === 'instances') {
    return route.path.startsWith('/instances');
  }
  return route.path === `/${section}`;
}
</script>

<template>
  <header class="control-header">
    <div class="control-header__content">
      <RouterLink class="control-brand" :to="{ name: 'dashboard' }" :aria-label="t('app.name')">
        <span class="brand-mark" aria-hidden="true">MN</span>
        <strong>{{ t('app.shortName') }}</strong>
      </RouterLink>

      <nav class="control-nav" :aria-label="t('nav.controlPanel')">
        <RouterLink :class="['control-nav__item', { active: isActive('dashboard') }]" :to="{ name: 'dashboard' }">
          <IconDashboard />
          <span>{{ t('nav.dashboard') }}</span>
        </RouterLink>
        <RouterLink :class="['control-nav__item', { active: isActive('instances') }]" :to="{ name: 'instances' }">
          <IconApps />
          <span>{{ t('nav.instances') }}</span>
        </RouterLink>
        <RouterLink :class="['control-nav__item', { active: isActive('nodes') }]" :to="{ name: 'nodes' }">
          <IconCloud />
          <span>{{ t('nav.nodes') }}</span>
        </RouterLink>
        <RouterLink :class="['control-nav__item', { active: isActive('settings') }]" :to="{ name: 'settings' }">
          <IconSettings />
          <span>{{ t('nav.settings') }}</span>
        </RouterLink>
      </nav>

      <div class="control-tools">
        <span class="core-health">
          <i :class="{ online: onlineCoreCount > 0 }"></i>
          {{ onlineCoreCount }}/{{ cores.length }} Core
        </span>
        <a-tooltip :content="t('common.refresh')">
          <a-button
            class="header-icon-button refresh-button"
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
        <a-tooltip :content="t('nav.logout')">
          <a-button
            class="header-icon-button"
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
    </div>
  </header>
</template>

<style scoped>
.control-header {
  position: sticky;
  z-index: 20;
  top: 0;
  height: 3.5rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-header);
}

.control-header__content {
  display: grid;
  align-items: center;
  width: min(100%, 108rem);
  height: 100%;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 1.35rem;
  margin: 0 auto;
  padding: 0 1.5rem;
}

.control-brand,
.control-nav,
.control-nav__item,
.control-tools,
.core-health,
.user-identity {
  display: flex;
  align-items: center;
}

.control-brand {
  gap: 0.65rem;
  color: var(--mcnp-text);
  text-decoration: none;
}

.brand-mark {
  display: inline-grid;
  width: 2rem;
  height: 2rem;
  border-radius: 4px;
  place-items: center;
  background: var(--mcnp-primary);
  color: #fff;
  font-size: 0.7rem;
  font-weight: 800;
}

.control-brand strong {
  font-size: 0.9rem;
}

.control-nav {
  align-self: stretch;
  min-width: 0;
}

.control-nav__item {
  gap: 0.45rem;
  border-bottom: 2px solid transparent;
  padding: 0 0.8rem;
  color: var(--mcnp-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
  text-decoration: none;
}

.control-nav__item:hover,
.control-nav__item.active {
  color: var(--mcnp-text);
}

.control-nav__item.active {
  border-bottom-color: var(--mcnp-primary);
}

.control-tools {
  min-width: 0;
  gap: 0.25rem;
}

.core-health {
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

.core-health i.online {
  background: var(--mcnp-success);
}

.header-icon-button {
  width: 2rem;
  height: 2rem;
  color: var(--mcnp-text-muted);
}

.user-identity {
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

@media (max-width: 52rem) {
  .control-header__content {
    gap: 0.6rem;
    padding: 0 0.75rem;
  }

  .control-brand strong,
  .control-nav__item span,
  .core-health,
  .user-name {
    display: none;
  }

  .control-nav__item {
    padding: 0 0.65rem;
    font-size: 1rem;
  }

  .user-identity {
    padding-left: 0.4rem;
  }
}

@media (max-width: 30rem) {
  .control-header__content {
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.2rem;
    padding: 0 0.45rem;
  }

  .control-nav {
    justify-self: center;
  }

  .control-nav__item {
    padding: 0 0.45rem;
  }

  .control-tools {
    display: flex;
    gap: 0.1rem;
  }

  .refresh-button,
  .user-identity {
    display: none;
  }
}
</style>
