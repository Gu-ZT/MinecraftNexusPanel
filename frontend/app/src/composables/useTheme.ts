import { computed, readonly, ref, watch } from 'vue';

export type ThemePreference = 'system' | 'light' | 'dark';
export type EffectiveTheme = Exclude<ThemePreference, 'system'>;

const THEME_STORAGE_KEY = 'mcnp.theme';
const colorSchemeQuery = window.matchMedia('(prefers-color-scheme: dark)');
const storedPreference = localStorage.getItem(THEME_STORAGE_KEY);
const themePreference = ref<ThemePreference>(isThemePreference(storedPreference) ? storedPreference : 'system');
const systemUsesDarkTheme = ref(colorSchemeQuery.matches);
const effectiveTheme = computed<EffectiveTheme>(() => {
  if (themePreference.value === 'system') {
    return systemUsesDarkTheme.value ? 'dark' : 'light';
  }
  return themePreference.value;
});
let initialized = false;

export function initializeTheme(): void {
  if (initialized) {
    return;
  }
  initialized = true;
  colorSchemeQuery.addEventListener('change', (event) => {
    systemUsesDarkTheme.value = event.matches;
  });
  watch(effectiveTheme, applyTheme, { immediate: true });
}

export function useTheme() {
  return {
    effectiveTheme: readonly(effectiveTheme),
    themePreference: readonly(themePreference),
    setThemePreference,
  };
}

function setThemePreference(preference: ThemePreference): void {
  themePreference.value = preference;
  localStorage.setItem(THEME_STORAGE_KEY, preference);
}

function applyTheme(theme: EffectiveTheme): void {
  const dark = theme === 'dark';
  if (dark) {
    document.documentElement.setAttribute('arco-theme', 'dark');
    document.body.setAttribute('arco-theme', 'dark');
  } else {
    document.documentElement.removeAttribute('arco-theme');
    document.body.removeAttribute('arco-theme');
  }
  document.documentElement.style.colorScheme = theme;
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', dark ? '#1b1c1f' : '#ffffff');
}

function isThemePreference(value: string | null): value is ThemePreference {
  return value === 'system' || value === 'light' || value === 'dark';
}
