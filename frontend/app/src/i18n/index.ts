import { ref } from 'vue';
import { createI18n } from 'vue-i18n';

interface LocaleMetadata {
  name?: string;
  order?: number;
}

type LocaleDocument = Record<string, unknown> & {
  $meta?: LocaleMetadata;
};

type MessageDictionary = {
  [key: string]: string | MessageDictionary;
};

export interface LocaleOption {
  code: string;
  name: string;
}

const LOCALE_STORAGE_KEY = 'mcnp.locale';
const localeModules = import.meta.glob('../locales/*.json', {
  eager: true,
  import: 'default',
}) as Record<string, LocaleDocument>;
const localeDocuments = Object.entries(localeModules)
  .map(([path, document]) => ({ code: localeCodeFromPath(path), document }))
  .sort((left, right) => (left.document.$meta?.order ?? 100) - (right.document.$meta?.order ?? 100));

export const availableLocales: LocaleOption[] = localeDocuments.map(({ code, document }) => ({
  code,
  name: document.$meta?.name ?? code,
}));

const messages = Object.fromEntries(
  localeDocuments.map(({ code, document }) => {
    const { $meta: _, ...localeMessages } = document;
    return [code, localeMessages];
  }),
) as unknown as Record<string, MessageDictionary>;
const storedPreference = localStorage.getItem(LOCALE_STORAGE_KEY);
export const localePreference = ref(
  storedPreference === 'auto' || availableLocales.some(({ code }) => code === storedPreference)
    ? (storedPreference ?? 'auto')
    : 'auto',
);
const initialLocale = resolveLocale(localePreference.value);
const fallbackLocale = availableLocales.find(({ code }) => code === 'zh-CN')?.code ?? availableLocales[0]?.code ?? 'zh-CN';

export const i18n = createI18n({
  legacy: false as const,
  locale: initialLocale,
  fallbackLocale,
  messages,
});

document.documentElement.lang = initialLocale;

export function setLocalePreference(preference: string): void {
  if (preference !== 'auto' && !availableLocales.some(({ code }) => code === preference)) {
    return;
  }
  localePreference.value = preference;
  localStorage.setItem(LOCALE_STORAGE_KEY, preference);
  const locale = resolveLocale(preference);
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}

function resolveLocale(preference: string): string {
  if (preference !== 'auto' && availableLocales.some(({ code }) => code === preference)) {
    return preference;
  }

  const normalizedLocales = new Map(availableLocales.map(({ code }) => [code.toLocaleLowerCase(), code]));
  for (const browserLocale of navigator.languages) {
    const exactMatch = normalizedLocales.get(browserLocale.toLocaleLowerCase());
    if (exactMatch) {
      return exactMatch;
    }
    const language = browserLocale.split('-')[0]?.toLocaleLowerCase();
    const languageMatch = availableLocales.find(({ code }) => code.toLocaleLowerCase().split('-')[0] === language);
    if (languageMatch) {
      return languageMatch.code;
    }
  }

  return availableLocales.find(({ code }) => code === 'zh-CN')?.code ?? availableLocales[0]?.code ?? 'zh-CN';
}

function localeCodeFromPath(path: string): string {
  return path.split('/').at(-1)?.replace(/\.json$/u, '') ?? path;
}
