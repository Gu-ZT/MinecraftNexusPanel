import '@arco-design/web-vue/dist/arco.css';
import { VueQueryPlugin } from '@tanstack/vue-query';
import { createPinia } from 'pinia';
import { createApp } from 'vue';

import App from './App.vue';
import { initializeTheme } from './composables/useTheme';
import { i18n } from './i18n';
import { router } from './router';
import './styles/base.css';

initializeTheme();

createApp(App).use(createPinia()).use(VueQueryPlugin).use(i18n).use(router).mount('#app');
