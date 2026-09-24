/** 应用入口。 */

import { computed, createApp } from 'vue';
import { createPinia } from 'pinia';
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query';
import '@arco-design/web-vue/dist/arco.css';
import '@mcnp/ui/tokens.css';
import '@/styles.css';
import App from '@/App.vue';
import { router } from '@/router';
import { PermissionKey } from '@mcnp/ui';
import { useAuthStore } from '@/stores/auth';

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
      staleTime: 10_000,
    },
  },
});
app.use(VueQueryPlugin, { queryClient });

// 权限上下文（PermissionGate 依赖）
const auth = useAuthStore();
app.provide(PermissionKey, {
  permissions: computed(() => auth.permissions),
  has: auth.has,
  hasAll: auth.hasAll,
});

app.use(router);
app.mount('#app');
