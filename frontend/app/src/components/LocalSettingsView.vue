<script setup lang="ts">
import { Option as AOption, Radio as ARadio, RadioGroup as ARadioGroup, Select as ASelect } from '@arco-design/web-vue';
import { IconComputer, IconLanguage, IconMoon, IconSettings, IconSun } from '@arco-design/web-vue/es/icon';
import { useI18n } from 'vue-i18n';

import type { PlatformKind } from '@mcnp/platform';

import type { ThemePreference } from '../composables/useTheme';
import { useTheme } from '../composables/useTheme';
import { availableLocales, localePreference, setLocalePreference } from '../i18n';

defineProps<{
  platformKind: PlatformKind;
  apiBaseUrl: string;
}>();

const { t } = useI18n();
const { effectiveTheme, themePreference, setThemePreference } = useTheme();

function changeTheme(value: string | number | boolean): void {
  if (value === 'system' || value === 'light' || value === 'dark') {
    setThemePreference(value satisfies ThemePreference);
  }
}

function changeLocale(value: unknown): void {
  if (typeof value === 'string') {
    setLocalePreference(value);
  }
}
</script>

<template>
  <main class="console-page settings-page">
    <header class="page-heading">
      <div>
        <p class="page-eyebrow"><IconSettings /> {{ t('settings.eyebrow') }}</p>
        <h1>{{ t('settings.title') }}</h1>
      </div>
      <p>{{ t('settings.summary') }}</p>
    </header>

    <section class="settings-section">
      <header>
        <span><IconSun /></span>
        <div>
          <h2>{{ t('settings.appearance') }}</h2>
          <p>{{ t('settings.appearanceHint') }}</p>
        </div>
      </header>
      <div class="settings-control">
        <div>
          <strong>{{ t('settings.theme') }}</strong>
          <small>{{ t('settings.effectiveTheme', { theme: t(`theme.${effectiveTheme}`) }) }}</small>
        </div>
        <a-radio-group :model-value="themePreference" type="button" @change="changeTheme">
          <a-radio value="system"><IconComputer /> {{ t('theme.system') }}</a-radio>
          <a-radio value="light"><IconSun /> {{ t('theme.light') }}</a-radio>
          <a-radio value="dark"><IconMoon /> {{ t('theme.dark') }}</a-radio>
        </a-radio-group>
      </div>
    </section>

    <section class="settings-section">
      <header>
        <span><IconLanguage /></span>
        <div>
          <h2>{{ t('settings.language') }}</h2>
          <p>{{ t('settings.languageHint') }}</p>
        </div>
      </header>
      <div class="settings-control">
        <div>
          <strong>{{ t('settings.displayLanguage') }}</strong>
          <small>{{ t('settings.localeFilesHint') }}</small>
        </div>
        <a-select :model-value="localePreference" class="locale-select" @change="changeLocale">
          <a-option value="auto">{{ t('language.auto') }}</a-option>
          <a-option v-for="option in availableLocales" :key="option.code" :value="option.code">
            {{ option.name }} · {{ option.code }}
          </a-option>
        </a-select>
      </div>
    </section>

    <section class="settings-section">
      <header>
        <span><IconComputer /></span>
        <div>
          <h2>{{ t('settings.runtime') }}</h2>
          <p>{{ t('settings.runtimeHint') }}</p>
        </div>
      </header>
      <dl class="runtime-details">
        <div><dt>{{ t('settings.platform') }}</dt><dd>{{ platformKind }}</dd></div>
        <div><dt>{{ t('settings.apiBaseUrl') }}</dt><dd>{{ apiBaseUrl || t('settings.sameOrigin') }}</dd></div>
      </dl>
    </section>
  </main>
</template>

<style scoped>
.settings-page {
  max-width: 70rem;
}

.settings-section {
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.settings-section + .settings-section {
  margin-top: 0.85rem;
}

.settings-section > header {
  display: grid;
  grid-template-columns: 2.3rem minmax(0, 1fr);
  align-items: center;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface-raised);
}

.settings-section > header > span {
  display: grid;
  width: 2.3rem;
  height: 2.3rem;
  border-radius: 5px;
  place-items: center;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
}

.settings-section header div,
.settings-control > div {
  display: grid;
  gap: 0.22rem;
}

.settings-section h2,
.settings-section p {
  margin: 0;
}

.settings-section h2 {
  color: var(--mcnp-text);
  font-size: 0.86rem;
}

.settings-section p,
.settings-control small {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
}

.settings-control {
  display: flex;
  min-height: 4.5rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1rem;
}

.settings-control strong {
  color: var(--mcnp-text);
  font-size: 0.75rem;
}

.settings-control :deep(.arco-radio-button-content) {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.locale-select {
  width: min(100%, 18rem);
}

.runtime-details {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
  margin: 0;
  padding: 1rem;
}

.runtime-details div {
  display: grid;
  min-width: 0;
  gap: 0.3rem;
}

.runtime-details dt {
  color: var(--mcnp-text-faint);
  font-size: 0.64rem;
}

.runtime-details dd {
  overflow-wrap: anywhere;
  margin: 0;
  color: var(--mcnp-text-muted);
  font-size: 0.72rem;
}

@media (max-width: 42rem) {
  .settings-control {
    align-items: stretch;
    flex-direction: column;
  }

  .runtime-details {
    grid-template-columns: 1fr;
  }
}
</style>
