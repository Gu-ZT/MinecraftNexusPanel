<script setup lang="ts">
import { Button as AButton, Doption as ADoption, Dropdown as ADropdown } from '@arco-design/web-vue';
import {
  IconCheck,
  IconComputer,
  IconLanguage,
  IconMoon,
  IconSun,
} from '@arco-design/web-vue/es/icon';
import { useI18n } from 'vue-i18n';

import type { ThemePreference } from '../composables/useTheme';
import { useTheme } from '../composables/useTheme';
import { availableLocales, localePreference, setLocalePreference } from '../i18n';

const { t } = useI18n();
const { effectiveTheme, themePreference, setThemePreference } = useTheme();

function changeTheme(value: string | number | Record<string, unknown> | undefined): void {
  if (value === 'system' || value === 'light' || value === 'dark') {
    setThemePreference(value satisfies ThemePreference);
  }
}

function changeLocale(value: string | number | Record<string, unknown> | undefined): void {
  if (typeof value === 'string') {
    setLocalePreference(value);
  }
}
</script>

<template>
  <div class="preference-controls">
    <a-dropdown trigger="click" position="br" @select="changeTheme">
      <a-button
        class="preference-button"
        type="text"
        size="small"
        :aria-label="t('theme.switch')"
        :title="t('theme.switch')"
      >
        <template #icon>
          <IconComputer v-if="themePreference === 'system'" />
          <IconMoon v-else-if="effectiveTheme === 'dark'" />
          <IconSun v-else />
        </template>
      </a-button>
      <template #content>
        <a-doption value="system">
          <span class="preference-option">
            <IconComputer />
            <span>{{ t('theme.system') }}</span>
            <IconCheck v-if="themePreference === 'system'" class="option-check" />
          </span>
        </a-doption>
        <a-doption value="light">
          <span class="preference-option">
            <IconSun />
            <span>{{ t('theme.light') }}</span>
            <IconCheck v-if="themePreference === 'light'" class="option-check" />
          </span>
        </a-doption>
        <a-doption value="dark">
          <span class="preference-option">
            <IconMoon />
            <span>{{ t('theme.dark') }}</span>
            <IconCheck v-if="themePreference === 'dark'" class="option-check" />
          </span>
        </a-doption>
      </template>
    </a-dropdown>

    <a-dropdown trigger="click" position="br" @select="changeLocale">
      <a-button
        class="preference-button"
        type="text"
        size="small"
        :aria-label="t('language.switch')"
        :title="t('language.switch')"
      >
        <template #icon><IconLanguage /></template>
      </a-button>
      <template #content>
        <a-doption value="auto">
          <span class="preference-option">
            <IconComputer />
            <span>{{ t('language.auto') }}</span>
            <IconCheck v-if="localePreference === 'auto'" class="option-check" />
          </span>
        </a-doption>
        <a-doption v-for="option in availableLocales" :key="option.code" :value="option.code">
          <span class="preference-option">
            <IconLanguage />
            <span>{{ option.name }}</span>
            <IconCheck v-if="localePreference === option.code" class="option-check" />
          </span>
        </a-doption>
      </template>
    </a-dropdown>
  </div>
</template>

<style scoped>
.preference-controls {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.preference-button {
  width: 2rem;
  height: 2rem;
  color: var(--mcnp-text-muted);
}

.preference-button:hover {
  color: var(--mcnp-text);
}

.preference-option {
  display: grid;
  grid-template-columns: 1rem minmax(7.5rem, 1fr) 1rem;
  align-items: center;
  gap: 0.55rem;
}

.option-check {
  color: var(--mcnp-primary);
}
</style>
