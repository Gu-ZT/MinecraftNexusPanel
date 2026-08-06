import { createRouter, createWebHistory } from 'vue-router';

import WorkspaceView from '../views/WorkspaceView.vue';

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/instances',
    },
    {
      path: '/instances',
      name: 'instances',
      component: WorkspaceView,
    },
    {
      path: '/instances/:coreId',
      name: 'core-instances',
      component: WorkspaceView,
    },
    {
      path: '/instances/:coreId/:instanceId/:view(console|config)?',
      name: 'instance-workspace',
      component: WorkspaceView,
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: '/instances',
    },
  ],
});
