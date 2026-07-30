import { defineStore } from 'pinia';

import { browserPlatform } from '@mcnp/platform';

export const useApplicationStore = defineStore('application', {
  state: () => ({
    platform: browserPlatform,
  }),
});
