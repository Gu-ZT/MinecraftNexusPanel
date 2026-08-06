import { createRouter, createWebHistory } from 'vue-router';

import WorkspaceView from '../views/WorkspaceView.vue';

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/dashboard',
    },
    {
      path: '/dashboard',
      name: 'dashboard',
      component: WorkspaceView,
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
      path: '/instances/:coreId/:instanceId/:view(overview|console|config|files)?',
      name: 'instance-workspace',
      component: WorkspaceView,
    },
    {
      path: '/nodes',
      name: 'nodes',
      component: WorkspaceView,
    },
    {
      path: '/users',
      name: 'users',
      component: WorkspaceView,
    },
    {
      path: '/settings',
      name: 'settings',
      component: WorkspaceView,
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: '/dashboard',
    },
  ],
});
