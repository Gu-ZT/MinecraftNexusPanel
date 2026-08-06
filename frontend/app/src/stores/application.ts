import { defineStore } from 'pinia';

import { browserPlatform, desktopPlatform } from '@mcnp/platform';

function isTauriDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export const useApplicationStore = defineStore('application', {
  state: () => ({
    platform: isTauriDesktop() ? desktopPlatform : browserPlatform,
  }),
});
