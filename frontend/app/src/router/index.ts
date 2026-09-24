/** 路由表与守卫：鉴权、初始化流程与权限点过滤。 */

import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';
import { Message } from '@arco-design/web-vue';
import type { Permission } from '@mcnp/api-client';
import { useAuthStore } from '@/stores/auth';
import { useApi, usePlatform } from '@/composables';

declare module 'vue-router' {
  interface RouteMeta {
    title?: string;
    /** 访问该路由需要的权限点（全部满足）。 */
    permission?: Permission[];
    /** 免登录页面（登录/初始化）。 */
    blank?: boolean;
  }
}

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('@/pages/LoginPage.vue'),
    meta: { title: '登录', blank: true },
  },
  {
    path: '/setup',
    name: 'setup',
    component: () => import('@/pages/SetupPage.vue'),
    meta: { title: '初始化管理员', blank: true },
  },
  {
    path: '/',
    component: () => import('@/pages/AppLayout.vue'),
    children: [
      { path: '', name: 'dashboard', component: () => import('@/pages/DashboardPage.vue'), meta: { title: '总览' } },
      {
        path: 'cores',
        name: 'cores',
        component: () => import('@/pages/cores/CoreListPage.vue'),
        meta: { title: 'Core 节点', permission: ['core.read'] },
      },
      {
        path: 'cores/:id',
        name: 'core-detail',
        component: () => import('@/pages/cores/CoreDetailPage.vue'),
        meta: { title: '节点详情', permission: ['core.read'] },
      },
      {
        path: 'instances',
        name: 'instances',
        component: () => import('@/pages/instances/InstanceListPage.vue'),
        meta: { title: '实例', permission: ['instance.read'] },
      },
      {
        path: 'instances/new',
        name: 'instance-new',
        component: () => import('@/pages/instances/InstanceCreatePage.vue'),
        meta: { title: '一键搭建', permission: ['instance.create'] },
      },
      {
        path: 'instances/:id',
        component: () => import('@/pages/instances/InstanceDetailLayout.vue'),
        meta: { permission: ['instance.read'] },
        children: [
          { path: '', redirect: (to) => ({ name: 'instance-console', params: to.params }) },
          {
            path: 'console',
            name: 'instance-console',
            component: () => import('@/pages/instances/tabs/InstanceConsolePage.vue'),
            meta: { title: '终端', permission: ['instance.console.read'] },
          },
          {
            path: 'files',
            name: 'instance-files',
            component: () => import('@/pages/instances/tabs/InstanceFilesPage.vue'),
            meta: { title: '文件', permission: ['file.read'] },
          },
          {
            path: 'configs',
            name: 'instance-configs',
            component: () => import('@/pages/instances/tabs/InstanceConfigsPage.vue'),
            meta: { title: '配置', permission: ['config.read'] },
          },
          {
            path: 'extensions',
            name: 'instance-extensions',
            component: () => import('@/pages/instances/tabs/InstanceExtensionsPage.vue'),
            meta: { title: '模组/插件', permission: ['extension.read'] },
          },
          {
            path: 'schedules',
            name: 'instance-schedules',
            component: () => import('@/pages/instances/tabs/InstanceSchedulesPage.vue'),
            meta: { title: '计划任务', permission: ['schedule.read'] },
          },
          {
            path: 'backups',
            name: 'instance-backups',
            component: () => import('@/pages/instances/tabs/InstanceBackupsPage.vue'),
            meta: { title: '备份', permission: ['instance.read'] },
          },
          {
            path: 'settings',
            name: 'instance-settings',
            component: () => import('@/pages/instances/tabs/InstanceSettingsPage.vue'),
            meta: { title: '设置', permission: ['instance.read'] },
          },
        ],
      },
      {
        path: 'environments',
        name: 'environments',
        component: () => import('@/pages/EnvironmentsPage.vue'),
        meta: { title: '环境管理', permission: ['environment.read'] },
      },
      {
        path: 'images',
        name: 'images',
        component: () => import('@/pages/images/ImageListPage.vue'),
        meta: { title: '镜像管理', permission: ['image.read'] },
      },
      {
        path: 'images/builds/:buildId',
        name: 'image-build-log',
        component: () => import('@/pages/images/ImageBuildLogPage.vue'),
        meta: { title: '构建日志', permission: ['image.read'] },
      },
      { path: 'tasks', name: 'tasks', component: () => import('@/pages/TasksPage.vue'), meta: { title: '任务中心' } },
      {
        path: 'users',
        name: 'users',
        component: () => import('@/pages/users/UserListPage.vue'),
        meta: { title: '用户', permission: ['user.read'] },
      },
      {
        path: 'users/groups',
        name: 'groups',
        component: () => import('@/pages/users/GroupListPage.vue'),
        meta: { title: '用户组', permission: ['user.read'] },
      },
      {
        path: 'audit',
        name: 'audit',
        component: () => import('@/pages/AuditPage.vue'),
        meta: { title: '审计日志', permission: ['audit.read'] },
      },
      { path: 'settings', name: 'settings', component: () => import('@/pages/SettingsPage.vue'), meta: { title: '个人设置' } },
    ],
  },
  { path: '/:pathMatch(.*)*', redirect: '/' },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});

let sessionRestored = false;

router.beforeEach(async (to) => {
  const auth = useAuthStore();
  const api = useApi();
  const platform = usePlatform();

  // 首次导航时尝试恢复会话（有本地令牌快照才发起 me 请求）
  if (!sessionRestored) {
    sessionRestored = true;
    if (platform.authStorage.read()) {
      try {
        const session = await api.auth.me();
        if (session) auth.applySession(session);
        else auth.clear(platform);
      } catch (err) {
        console.warn('[router] 会话恢复失败', err);
        auth.clear(platform);
      }
    }
  }

  if (to.meta.blank) {
    if (to.name === 'setup') {
      const required = await api.auth.setupRequired();
      if (!required) return { name: 'login' };
    }
    if (to.name === 'login' && auth.isLoggedIn) return { path: '/' };
    return true;
  }

  if (!auth.isLoggedIn) return { name: 'login', query: { redirect: to.fullPath } };

  if (to.meta.permission && !auth.hasAll(to.meta.permission)) {
    Message.warning(`没有访问「${to.meta.title ?? to.path}」的权限`);
    return { path: '/' };
  }
  return true;
});

router.afterEach((to) => {
  document.title = to.meta.title ? `${to.meta.title} · Minecraft Nexus Panel` : 'Minecraft Nexus Panel';
});
