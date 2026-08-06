<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  InputPassword as AInputPassword,
  Spin as ASpin,
} from '@arco-design/web-vue';
import {
  IconApps,
  IconCloud,
  IconCode,
  IconCommand,
  IconExport,
  IconPlayArrow,
  IconPoweroff,
  IconRefresh,
  IconRight,
  IconSearch,
  IconSend,
  IconSettings,
  IconStop,
  IconUser,
} from '@arco-design/web-vue/es/icon';
import { computed, onMounted, ref, shallowRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RouteLocationRaw } from 'vue-router';
import { useRoute, useRouter } from 'vue-router';

import { ApiRequestError, createPanelApiClient } from '@mcnp/api-client';
import type { Core, Instance, InstanceState, LogLine, PanelApiClient, User } from '@mcnp/api-client';
import type { DesktopRuntimeInfo } from '@mcnp/platform';

import ConfigEditor from '../components/ConfigEditor.vue';
import PreferenceControls from '../components/PreferenceControls.vue';
import { useApplicationStore } from '../stores/application';

type ActiveView = 'console' | 'config';
type NavigationMode = 'push' | 'replace';

const CSRF_STORAGE_KEY = 'mcnp.csrfToken';
const ACCESS_TOKEN_STORAGE_KEY = 'mcnp.accessToken';

const { locale, t, te } = useI18n();
const route = useRoute();
const router = useRouter();
const application = useApplicationStore();
const username = ref('');
const password = ref('');
const csrfToken = ref(sessionStorage.getItem(CSRF_STORAGE_KEY) ?? '');
const accessToken = ref(sessionStorage.getItem(ACCESS_TOKEN_STORAGE_KEY) ?? '');
const desktopRuntime = ref<DesktopRuntimeInfo | null>(null);
const currentUser = ref<User | null>(null);
const cores = ref<Core[]>([]);
const selectedCoreId = ref('');
const instances = ref<Instance[]>([]);
const selectedInstanceId = ref('');
const logs = ref<LogLine[]>([]);
const command = ref('');
const instanceQuery = ref('');
const loading = ref(false);
const instancesLoading = ref(false);
const logsLoading = ref(false);
const loginPending = ref(false);
const actionPending = ref<string | null>(null);
const errorMessage = ref('');
const noticeMessage = ref('');
const activeView = ref<ActiveView>('console');
const panelApiClient = shallowRef<PanelApiClient>(
  createPanelApiClient({
    baseUrl: application.platform.apiBaseUrl,
    getAccessToken: () => accessToken.value || undefined,
    getCsrfToken: () => csrfToken.value || undefined,
  }),
);
let syncingRoute = false;
let applyingRoute = false;

const selectedInstance = computed(
  () => instances.value.find((instance) => instance.id === selectedInstanceId.value) ?? null,
);
const filteredInstances = computed(() => {
  const query = instanceQuery.value.trim().toLocaleLowerCase();
  if (!query) {
    return instances.value;
  }
  return instances.value.filter((instance) =>
    [instance.name, instance.id, instance.kind].some((value) => value.toLocaleLowerCase().includes(query)),
  );
});
const onlineCoreCount = computed(() => cores.value.filter((core) => core.status === 'ONLINE').length);
const runningInstanceCount = computed(
  () => instances.value.filter((instance) => instance.runtime.state === 'RUNNING').length,
);
const userInitial = computed(() => currentUser.value?.displayName.trim().charAt(0).toLocaleUpperCase() || 'U');
const canStartSelectedInstance = computed(() => canStart(selectedInstance.value?.runtime.state));
const canStopSelectedInstance = computed(() => canStop(selectedInstance.value?.runtime.state));

onMounted(() => {
  void restoreSession();
});

watch(
  () => route.fullPath,
  () => {
    if (currentUser.value && !syncingRoute) {
      void applyRouteSelection();
    }
  },
);

async function restoreSession(): Promise<void> {
  try {
    if (application.platform.initialize) {
      desktopRuntime.value = await application.platform.initialize();
      panelApiClient.value = createPanelApiClient({
        baseUrl: desktopRuntime.value.apiBaseUrl,
        getAccessToken: () => accessToken.value || undefined,
        getCsrfToken: () => csrfToken.value || undefined,
      });
      if (!username.value && desktopRuntime.value.initialAdminUsername) {
        username.value = desktopRuntime.value.initialAdminUsername;
      }
    }
  } catch (error) {
    if (application.platform.kind === 'desktop') {
      errorMessage.value = describeError(error, t('error.desktopStart'));
    }
    clearSession();
    return;
  }

  try {
    currentUser.value = await api().getCurrentUser();
    await loadWorkspace();
  } catch {
    clearSession();
  }
}

async function signIn(): Promise<void> {
  loginPending.value = true;
  errorMessage.value = '';
  try {
    const clientType = application.platform.kind === 'desktop' ? 'NATIVE' : 'BROWSER';
    const response = await api().login(username.value.trim(), password.value, clientType);
    currentUser.value = response.user;
    accessToken.value = response.session.accessToken ?? '';
    if (accessToken.value) {
      sessionStorage.setItem(ACCESS_TOKEN_STORAGE_KEY, accessToken.value);
    } else {
      sessionStorage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
    }
    csrfToken.value = response.session.csrfToken ?? '';
    sessionStorage.setItem(CSRF_STORAGE_KEY, csrfToken.value);
    password.value = '';
    if (desktopRuntime.value?.initialAdminPassword && application.platform.completeInitialAdmin) {
      try {
        await application.platform.completeInitialAdmin();
        desktopRuntime.value = { ...desktopRuntime.value, initialAdminPassword: null };
      } catch (error) {
        noticeMessage.value = describeError(error, t('error.bootstrapCleanup'));
      }
    }
    try {
      await loadWorkspace();
    } catch (error) {
      errorMessage.value = describeError(error, t('error.workspaceAfterLogin'));
    }
  } catch (error) {
    errorMessage.value = describeError(error, t('error.login'));
  } finally {
    loginPending.value = false;
  }
}

async function signOut(): Promise<void> {
  actionPending.value = 'logout';
  try {
    await api().logout();
  } catch {
    // 本地会话始终清理，避免失联的 Panel 阻止用户退出当前设备。
  } finally {
    actionPending.value = null;
    clearSession();
    await router.replace({ name: 'instances' });
  }
}

async function refreshWorkspace(): Promise<void> {
  await loadWorkspace();
}

async function loadWorkspace(): Promise<void> {
  loading.value = true;
  errorMessage.value = '';
  try {
    const page = await api().listCores();
    cores.value = page.items;
    const requestedCoreId = routeParam('coreId');
    if (cores.value.some((core) => core.id === requestedCoreId)) {
      selectedCoreId.value = requestedCoreId;
    } else if (!cores.value.some((core) => core.id === selectedCoreId.value)) {
      selectedCoreId.value = cores.value[0]?.id ?? '';
    }
    await loadInstances(true);
    await syncWorkspaceRoute('replace');
  } catch (error) {
    errorMessage.value = describeError(error, t('error.workspace'));
  } finally {
    loading.value = false;
  }
}

async function selectCore(coreId: string): Promise<void> {
  if (coreId === selectedCoreId.value) {
    return;
  }
  selectedCoreId.value = coreId;
  selectedInstanceId.value = '';
  instanceQuery.value = '';
  activeView.value = 'console';
  await loadInstances(false);
  await syncWorkspaceRoute('push');
}

async function loadInstances(preferRoute: boolean): Promise<void> {
  logs.value = [];
  instances.value = [];
  if (!selectedCoreId.value) {
    selectedInstanceId.value = '';
    return;
  }

  instancesLoading.value = true;
  try {
    const page = await api().listInstances(selectedCoreId.value);
    instances.value = page.items;
    const requestedInstanceId =
      preferRoute && routeParam('coreId') === selectedCoreId.value ? routeParam('instanceId') : '';
    if (instances.value.some((instance) => instance.id === requestedInstanceId)) {
      selectedInstanceId.value = requestedInstanceId;
    } else if (!instances.value.some((instance) => instance.id === selectedInstanceId.value)) {
      selectedInstanceId.value = instances.value[0]?.id ?? '';
    }
    activeView.value =
      preferRoute && selectedInstanceId.value && routeParam('view') === 'config' ? 'config' : activeView.value;
    if (!selectedInstanceId.value) {
      activeView.value = 'console';
    }
    await loadLogs();
  } catch (error) {
    selectedInstanceId.value = '';
    errorMessage.value = describeError(error, t('error.instances'));
  } finally {
    instancesLoading.value = false;
  }
}

async function selectInstance(instanceId: string): Promise<void> {
  if (instanceId === selectedInstanceId.value) {
    return;
  }
  selectedInstanceId.value = instanceId;
  activeView.value = 'console';
  await loadLogs();
  await syncWorkspaceRoute('push');
}

async function changeActiveView(view: ActiveView): Promise<void> {
  if (view === 'config' && !selectedInstance.value) {
    return;
  }
  activeView.value = view;
  await syncWorkspaceRoute('push');
}

async function applyRouteSelection(): Promise<void> {
  if (applyingRoute || cores.value.length === 0) {
    return;
  }
  applyingRoute = true;
  try {
    const requestedCoreId = routeParam('coreId');
    const nextCoreId = cores.value.some((core) => core.id === requestedCoreId)
      ? requestedCoreId
      : (cores.value[0]?.id ?? '');
    if (nextCoreId !== selectedCoreId.value) {
      selectedCoreId.value = nextCoreId;
      selectedInstanceId.value = '';
      instanceQuery.value = '';
      activeView.value = 'console';
      await loadInstances(true);
    } else {
      const requestedInstanceId = routeParam('instanceId');
      const nextInstanceId = instances.value.some((instance) => instance.id === requestedInstanceId)
        ? requestedInstanceId
        : (instances.value[0]?.id ?? '');
      const selectionChanged = nextInstanceId !== selectedInstanceId.value;
      selectedInstanceId.value = nextInstanceId;
      activeView.value = nextInstanceId && routeParam('view') === 'config' ? 'config' : 'console';
      if (selectionChanged) {
        await loadLogs();
      }
    }
    await syncWorkspaceRoute('replace');
  } finally {
    applyingRoute = false;
  }
}

async function loadLogs(): Promise<void> {
  if (!selectedCoreId.value || !selectedInstanceId.value) {
    logs.value = [];
    return;
  }

  logsLoading.value = true;
  try {
    const page = await api().getInstanceLogs(selectedCoreId.value, selectedInstanceId.value);
    logs.value = page.items;
  } catch (error) {
    logs.value = [];
    errorMessage.value = describeError(error, t('error.logs'));
  } finally {
    logsLoading.value = false;
  }
}

async function startInstance(): Promise<void> {
  await runInstanceAction('start', () => api().startInstance(selectedCoreId.value, selectedInstanceId.value));
}

async function stopInstance(): Promise<void> {
  await runInstanceAction('stop', () => api().stopInstance(selectedCoreId.value, selectedInstanceId.value));
}

async function killInstance(): Promise<void> {
  await runInstanceAction('kill', () => api().killInstance(selectedCoreId.value, selectedInstanceId.value));
}

async function sendCommand(): Promise<void> {
  const trimmedCommand = command.value.trim();
  if (!trimmedCommand || !selectedCoreId.value || !selectedInstanceId.value) {
    return;
  }

  actionPending.value = 'command';
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    await api().sendInstanceCommand(selectedCoreId.value, selectedInstanceId.value, trimmedCommand);
    command.value = '';
    noticeMessage.value = t('notice.commandSent');
    await loadLogs();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.command'));
  } finally {
    actionPending.value = null;
  }
}

async function runInstanceAction(action: string, operation: () => Promise<unknown>): Promise<void> {
  if (!selectedCoreId.value || !selectedInstanceId.value) {
    return;
  }

  actionPending.value = action;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    await operation();
    noticeMessage.value = actionNotice(action);
    await loadInstances(false);
    await syncWorkspaceRoute('replace');
  } catch (error) {
    errorMessage.value = describeError(error, t('error.instanceAction'));
  } finally {
    actionPending.value = null;
  }
}

async function syncWorkspaceRoute(mode: NavigationMode): Promise<void> {
  const target = workspaceRoute();
  if (router.resolve(target).fullPath === route.fullPath) {
    return;
  }
  syncingRoute = true;
  try {
    if (mode === 'push') {
      await router.push(target);
    } else {
      await router.replace(target);
    }
  } finally {
    syncingRoute = false;
  }
}

function workspaceRoute(): RouteLocationRaw {
  if (selectedCoreId.value && selectedInstanceId.value) {
    return {
      name: 'instance-workspace',
      params: {
        coreId: selectedCoreId.value,
        instanceId: selectedInstanceId.value,
        view: activeView.value,
      },
    };
  }
  if (selectedCoreId.value) {
    return { name: 'core-instances', params: { coreId: selectedCoreId.value } };
  }
  return { name: 'instances' };
}

function routeParam(name: string): string {
  const value = route.params[name];
  return typeof value === 'string' ? value : '';
}

function api(): PanelApiClient {
  return panelApiClient.value;
}

function clearSession(): void {
  sessionStorage.removeItem(CSRF_STORAGE_KEY);
  sessionStorage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
  csrfToken.value = '';
  accessToken.value = '';
  currentUser.value = null;
  cores.value = [];
  instances.value = [];
  logs.value = [];
  instanceQuery.value = '';
  selectedCoreId.value = '';
  selectedInstanceId.value = '';
  activeView.value = 'console';
}

function describeError(error: unknown, fallback: string): string {
  if (error instanceof ApiRequestError || error instanceof Error) {
    return `${fallback}: ${error.message}`;
  }
  return fallback;
}

function canStart(state: InstanceState | undefined): boolean {
  return state === 'CREATED' || state === 'STOPPED' || state === 'FAILED' || state === 'UNKNOWN';
}

function canStop(state: InstanceState | undefined): boolean {
  return state === 'RUNNING' || state === 'STARTING';
}

function actionNotice(action: string): string {
  if (action === 'start') {
    return t('notice.startSubmitted');
  }
  if (action === 'stop') {
    return t('notice.stopSubmitted');
  }
  if (action === 'kill') {
    return t('notice.killSubmitted');
  }
  return t('notice.actionSubmitted');
}

function formatDate(value: string | null | undefined): string {
  if (!value) {
    return t('common.notRecorded');
  }
  return new Intl.DateTimeFormat(locale.value, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    month: '2-digit',
    day: '2-digit',
  }).format(new Date(value));
}

function statusClass(status: string): string {
  return `status status-${status.toLocaleLowerCase().replaceAll('_', '-')}`;
}

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}
</script>

<template>
  <form v-if="!currentUser" class="login-shell" @submit.prevent="signIn">
    <div class="login-utilities">
      <PreferenceControls />
    </div>
    <section class="login-panel">
      <div class="login-brand">
        <span class="brand-mark" aria-hidden="true">MN</span>
        <span>{{ t('app.name') }}</span>
      </div>
      <div class="login-heading">
        <p class="eyebrow">{{ t('auth.eyebrow') }}</p>
        <h1>{{ t('auth.title') }}</h1>
      </div>
      <div v-if="desktopRuntime?.initialAdminPassword" class="bootstrap-credentials">
        <strong>{{ t('auth.bootstrapTitle') }}</strong>
        <p>{{ t('auth.bootstrapHint') }}</p>
        <code>{{ t('auth.username') }}: {{ desktopRuntime.initialAdminUsername }}</code>
        <code>{{ t('auth.password') }}: {{ desktopRuntime.initialAdminPassword }}</code>
      </div>
      <label class="login-field">
        <span>{{ t('auth.username') }}</span>
        <a-input v-model="username" autocomplete="username" size="large" allow-clear>
          <template #prefix><IconUser /></template>
        </a-input>
      </label>
      <label class="login-field">
        <span>{{ t('auth.password') }}</span>
        <a-input-password v-model="password" autocomplete="current-password" size="large" allow-clear />
      </label>
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
      <a-button
        type="primary"
        size="large"
        long
        html-type="submit"
        :loading="loginPending"
        :disabled="!username || !password"
      >
        {{ loginPending ? t('auth.loggingIn') : t('auth.login') }}
      </a-button>
    </section>
  </form>

  <div v-else class="workspace">
    <header class="topbar">
      <div class="topbar-content">
        <div class="brand">
          <span class="brand-mark" aria-hidden="true">MN</span>
          <strong>{{ t('app.shortName') }}</strong>
        </div>
        <nav class="primary-nav" :aria-label="t('nav.workspace')">
          <span class="nav-item active" aria-current="page">
            <IconApps />
            {{ t('nav.instances') }}
          </span>
        </nav>
        <div class="topbar-tools">
          <span class="core-health">
            <i :class="{ online: onlineCoreCount > 0 }"></i>
            {{ onlineCoreCount }}/{{ cores.length }} Core
          </span>
          <a-button
            class="topbar-icon-button"
            type="text"
            size="small"
            :loading="loading"
            :aria-label="t('workspace.refresh')"
            :title="t('workspace.refresh')"
            @click="refreshWorkspace"
          >
            <template #icon><IconRefresh /></template>
          </a-button>
          <PreferenceControls />
          <div class="user-identity" :title="currentUser.username">
            <span class="user-avatar">{{ userInitial }}</span>
            <span class="user-name">{{ currentUser.displayName }}</span>
          </div>
          <a-button
            class="topbar-icon-button"
            type="text"
            size="small"
            :loading="actionPending === 'logout'"
            :aria-label="t('nav.logout')"
            :title="t('nav.logout')"
            @click="signOut"
          >
            <template #icon><IconExport /></template>
          </a-button>
        </div>
      </div>
    </header>

    <div class="workspace-page">
      <header class="page-header">
        <div class="breadcrumbs" :aria-label="t('nav.breadcrumb')">
          <span>{{ t('nav.controlPanel') }}</span>
          <IconRight />
          <span>{{ t('nav.instances') }}</span>
          <template v-if="selectedInstance">
            <IconRight />
            <strong>{{ selectedInstance.name }}</strong>
          </template>
        </div>
        <div class="page-title-row">
          <h1>{{ t('workspace.title') }}</h1>
          <div class="workspace-summary">
            <span>
              <i class="summary-dot core-dot"></i>
              {{ t('workspace.coreOnlineSummary', { online: onlineCoreCount, total: cores.length }) }}
            </span>
            <span>
              <i class="summary-dot instance-dot"></i>
              {{ t('workspace.instanceRunningSummary', { running: runningInstanceCount, total: instances.length }) }}
            </span>
          </div>
        </div>
      </header>

      <main class="workspace-grid">
        <aside class="core-panel" :aria-label="t('cores.title')">
          <div class="panel-heading">
            <span class="heading-icon"><IconCloud /></span>
            <h2>{{ t('cores.title') }}</h2>
            <span class="panel-count">{{ cores.length }}</span>
          </div>
          <div v-if="loading && cores.length === 0" class="panel-loading"><a-spin /></div>
          <a-empty v-else-if="cores.length === 0" class="compact-empty" :description="t('cores.empty')" />
          <div v-else class="core-list">
            <button
              v-for="core in cores"
              :key="core.id"
              class="core-item"
              :class="{ selected: core.id === selectedCoreId }"
              type="button"
              @click="selectCore(core.id)"
            >
              <span class="item-copy">
                <strong>{{ core.name }}</strong>
                <small>{{ core.address }}</small>
              </span>
              <span class="item-meta">
                <i :class="statusClass(core.status)">
                  <span></span>
                  {{ statusLabel(core.status) }}
                </i>
                <small v-if="core.latencyMs !== null">{{ t('cores.latency', { value: core.latencyMs }) }}</small>
              </span>
            </button>
          </div>
        </aside>

        <section class="instance-panel" :aria-label="t('instances.title')">
          <div class="panel-heading">
            <span class="heading-icon"><IconApps /></span>
            <h2>{{ t('instances.title') }}</h2>
            <span class="panel-count">{{ instances.length }}</span>
          </div>
          <div class="instance-search">
            <a-input
              v-model="instanceQuery"
              size="small"
              allow-clear
              :disabled="!selectedCoreId"
              :placeholder="t('instances.filterPlaceholder')"
            >
              <template #prefix><IconSearch /></template>
            </a-input>
          </div>
          <div v-if="instancesLoading" class="panel-loading"><a-spin /></div>
          <a-empty
            v-else-if="!selectedCoreId"
            class="compact-empty"
            :description="t('instances.selectCore')"
          />
          <a-empty
            v-else-if="instances.length === 0"
            class="compact-empty"
            :description="t('instances.empty')"
          />
          <a-empty
            v-else-if="filteredInstances.length === 0"
            class="compact-empty"
            :description="t('instances.noMatches')"
          />
          <div v-else class="instance-list">
            <button
              v-for="instance in filteredInstances"
              :key="instance.id"
              class="instance-item"
              :class="{ selected: instance.id === selectedInstanceId }"
              type="button"
              @click="selectInstance(instance.id)"
            >
              <span class="instance-state-mark" :class="statusClass(instance.runtime.state)"></span>
              <span class="item-copy">
                <strong>{{ instance.name }}</strong>
                <small>{{ t('instances.kindAndId', { id: instance.id, kind: instance.kind }) }}</small>
              </span>
              <i :class="statusClass(instance.runtime.state)">
                <span></span>
                {{ statusLabel(instance.runtime.state) }}
              </i>
            </button>
          </div>
        </section>

        <section class="console-panel" :aria-label="t('console.title')">
          <div class="console-head">
            <div class="console-title">
              <h2>{{ selectedInstance?.name ?? t('console.title') }}</h2>
              <p v-if="selectedInstance">
                {{
                  t('console.directoryAndPid', {
                    directory: selectedInstance.directory,
                    pid: selectedInstance.runtime.pid ?? t('console.notRunning'),
                  })
                }}
              </p>
            </div>
            <div class="actions">
              <a-button
                class="start-button"
                size="small"
                type="primary"
                :loading="actionPending === 'start'"
                :disabled="!selectedInstance || !canStartSelectedInstance || actionPending !== null"
                @click="startInstance"
              >
                <template #icon><IconPlayArrow /></template>
                {{ t('console.start') }}
              </a-button>
              <a-button
                size="small"
                :loading="actionPending === 'stop'"
                :disabled="!selectedInstance || !canStopSelectedInstance || actionPending !== null"
                @click="stopInstance"
              >
                <template #icon><IconStop /></template>
                {{ t('console.stop') }}
              </a-button>
              <a-button
                size="small"
                status="danger"
                :loading="actionPending === 'kill'"
                :disabled="!selectedInstance || actionPending !== null"
                @click="killInstance"
              >
                <template #icon><IconPoweroff /></template>
                {{ t('console.kill') }}
              </a-button>
            </div>
          </div>

          <div class="view-tabs" role="tablist" :aria-label="t('nav.workspace')">
            <button
              class="view-tab"
              :class="{ selected: activeView === 'console' }"
              type="button"
              role="tab"
              :aria-selected="activeView === 'console'"
              @click="changeActiveView('console')"
            >
              <IconCode />
              {{ t('console.consoleTab') }}
            </button>
            <button
              class="view-tab"
              :class="{ selected: activeView === 'config' }"
              type="button"
              role="tab"
              :aria-selected="activeView === 'config'"
              :disabled="!selectedInstance"
              @click="changeActiveView('config')"
            >
              <IconSettings />
              {{ t('console.configTab') }}
            </button>
          </div>

          <div v-if="activeView === 'console'" class="console-view">
            <div v-if="selectedInstance" class="runtime-strip">
              <div class="runtime-item">
                <span>{{ t('console.state') }}</span>
                <i :class="statusClass(selectedInstance.runtime.state)">
                  <span></span>
                  {{ statusLabel(selectedInstance.runtime.state) }}
                </i>
              </div>
              <div class="runtime-item">
                <span>{{ t('console.pid') }}</span>
                <strong>{{ selectedInstance.runtime.pid ?? t('common.none') }}</strong>
              </div>
              <div class="runtime-item">
                <span>{{ t('console.startedAt') }}</span>
                <strong>{{ formatDate(selectedInstance.runtime.startedAt) }}</strong>
              </div>
              <div class="runtime-item">
                <span>{{ t('console.exitCode') }}</span>
                <strong>{{ selectedInstance.runtime.exitCode ?? t('common.none') }}</strong>
              </div>
            </div>

            <p v-if="errorMessage" class="form-error workspace-message" role="alert">{{ errorMessage }}</p>
            <p v-else-if="noticeMessage" class="notice workspace-message" role="status">{{ noticeMessage }}</p>

            <div class="terminal-shell">
              <header class="terminal-toolbar">
                <span>
                  <IconCode />
                  {{ t('console.output') }}
                  <small>{{ logs.length }}</small>
                </span>
                <a-button
                  class="terminal-refresh"
                  type="text"
                  size="mini"
                  :loading="logsLoading"
                  :disabled="!selectedInstance"
                  :aria-label="t('console.refreshOutput')"
                  :title="t('console.refreshOutput')"
                  @click="loadLogs"
                >
                  <template #icon><IconRefresh /></template>
                </a-button>
              </header>
              <div class="log-pane">
                <p v-if="!selectedInstance" class="terminal-muted">{{ t('console.selectInstance') }}</p>
                <p v-else-if="logs.length === 0" class="terminal-muted">{{ t('console.emptyOutput') }}</p>
                <ol v-else>
                  <li v-for="line in logs" :key="line.cursor" :class="`stream-${line.stream}`">
                    <time>{{ formatDate(line.occurredAt) }}</time>
                    <span>{{ line.line }}</span>
                  </li>
                </ol>
              </div>
            </div>

            <form class="command-row" @submit.prevent="sendCommand">
              <a-input
                v-model="command"
                :disabled="!selectedInstance || actionPending !== null"
                :placeholder="t('console.commandPlaceholder')"
              >
                <template #prefix><IconCommand /></template>
              </a-input>
              <a-button
                type="primary"
                html-type="submit"
                :loading="actionPending === 'command'"
                :disabled="!selectedInstance || !command.trim() || actionPending !== null"
              >
                <template #icon><IconSend /></template>
                {{ t('common.send') }}
              </a-button>
            </form>
          </div>

          <ConfigEditor
            v-else-if="selectedInstance"
            :client="panelApiClient"
            :core-id="selectedCoreId"
            :instance-id="selectedInstance.id"
          />
          <p v-else class="muted config-empty">{{ t('console.selectForConfig') }}</p>
        </section>
      </main>
    </div>
  </div>
</template>

<style scoped>
h1,
h2,
p {
  margin: 0;
}

.login-shell {
  display: grid;
  min-height: 100vh;
  grid-template-rows: auto 1fr;
  padding: 1rem 1.25rem 3rem;
  background: var(--mcnp-bg);
}

.login-utilities {
  display: flex;
  justify-content: flex-end;
}

.login-panel {
  display: grid;
  align-self: center;
  justify-self: center;
  width: min(100%, 25rem);
  gap: 1.1rem;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  padding: 1.75rem;
  background: var(--mcnp-surface);
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.12);
}

.login-brand,
.brand {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.login-brand {
  color: var(--mcnp-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
}

.brand-mark {
  display: inline-grid;
  flex: 0 0 2rem;
  width: 2rem;
  height: 2rem;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 4px;
  place-items: center;
  background: var(--mcnp-primary);
  color: #ffffff;
  font-size: 0.7rem;
  font-weight: 800;
}

.login-heading {
  display: grid;
  gap: 0.35rem;
  padding: 0.4rem 0 0.1rem;
}

.eyebrow {
  color: var(--mcnp-primary);
  font-size: 0.72rem;
  font-weight: 700;
}

.login-heading h1 {
  color: var(--mcnp-text);
  font-size: 1.45rem;
  font-weight: 650;
  letter-spacing: 0;
}

.bootstrap-credentials {
  display: grid;
  gap: 0.4rem;
  border-left: 3px solid var(--mcnp-primary);
  padding: 0.75rem 0.85rem;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
}

.bootstrap-credentials strong {
  color: var(--mcnp-text);
}

.bootstrap-credentials p {
  line-height: 1.45;
}

.bootstrap-credentials code {
  overflow-wrap: anywhere;
  color: var(--mcnp-primary);
  font-family: "Cascadia Mono", Consolas, monospace;
}

.login-field {
  display: grid;
  gap: 0.45rem;
  color: var(--mcnp-text-muted);
  font-size: 0.82rem;
  font-weight: 600;
}

.workspace {
  height: 100vh;
  overflow: hidden;
  background: var(--mcnp-bg);
  color: var(--mcnp-text);
}

.topbar {
  height: 3.5rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-header);
}

.topbar-content {
  display: grid;
  align-items: center;
  width: min(100%, 108rem);
  height: 100%;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 1.5rem;
  margin: 0 auto;
  padding: 0 1.5rem;
}

.brand strong {
  color: var(--mcnp-text);
  font-size: 0.9rem;
}

.primary-nav {
  display: flex;
  align-self: stretch;
  min-width: 0;
}

.nav-item {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  border-bottom: 2px solid transparent;
  padding: 0 0.8rem;
  color: var(--mcnp-text-muted);
  font-size: 0.82rem;
  font-weight: 600;
}

.nav-item.active {
  border-bottom-color: var(--mcnp-primary);
  color: var(--mcnp-text);
}

.topbar-tools,
.user-identity,
.core-health {
  display: flex;
  align-items: center;
}

.topbar-tools {
  gap: 0.35rem;
}

.core-health {
  gap: 0.45rem;
  margin-right: 0.35rem;
  color: var(--mcnp-text-muted);
  font-size: 0.72rem;
  white-space: nowrap;
}

.core-health i,
.summary-dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: var(--mcnp-text-faint);
}

.core-health i.online,
.core-dot,
.instance-dot {
  background: var(--mcnp-success);
}

.topbar-icon-button {
  width: 2rem;
  height: 2rem;
  color: var(--mcnp-text-muted);
}

.topbar-icon-button:hover {
  color: var(--mcnp-text);
}

.user-identity {
  min-width: 0;
  gap: 0.5rem;
  margin-left: 0.35rem;
  padding-left: 0.7rem;
  border-left: 1px solid var(--mcnp-border);
}

.user-avatar {
  display: grid;
  flex: 0 0 1.75rem;
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 50%;
  place-items: center;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
  font-size: 0.72rem;
  font-weight: 700;
}

.user-name {
  max-width: 8rem;
  overflow: hidden;
  color: var(--mcnp-text-muted);
  font-size: 0.76rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-page {
  display: flex;
  width: min(100%, 108rem);
  height: calc(100vh - 3.5rem);
  flex-direction: column;
  gap: 0.9rem;
  margin: 0 auto;
  padding: 1rem 1.5rem 1.25rem;
  overflow: hidden;
}

.page-header {
  display: grid;
  flex: 0 0 auto;
  gap: 0.8rem;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 0.35rem;
  color: var(--mcnp-text-faint);
  font-size: 0.74rem;
}

.breadcrumbs svg {
  flex: 0 0 auto;
  font-size: 0.65rem;
}

.breadcrumbs strong {
  overflow: hidden;
  color: var(--mcnp-text-muted);
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.page-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-width: 0;
  gap: 1rem;
}

.page-title-row h1 {
  color: var(--mcnp-text);
  font-size: 1.15rem;
  font-weight: 650;
  letter-spacing: 0;
}

.workspace-summary {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 1rem;
  color: var(--mcnp-text-muted);
  font-size: 0.74rem;
}

.workspace-summary span {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.instance-dot {
  background: var(--mcnp-primary);
}

.workspace-grid {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: 14rem 19rem minmax(28rem, 1fr);
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.core-panel,
.instance-panel,
.console-panel {
  min-width: 0;
  min-height: 0;
  background: var(--mcnp-surface);
}

.core-panel,
.instance-panel {
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--mcnp-border);
}

.console-panel {
  display: flex;
  flex-direction: column;
}

.panel-heading {
  display: grid;
  align-items: center;
  min-height: 3.25rem;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 0.55rem;
  padding: 0 0.85rem;
  border-bottom: 1px solid var(--mcnp-border);
}

.heading-icon {
  display: grid;
  width: 1.5rem;
  height: 1.5rem;
  place-items: center;
  color: var(--mcnp-text-faint);
  font-size: 0.9rem;
}

.panel-heading h2,
.console-head h2 {
  overflow: hidden;
  color: var(--mcnp-text);
  font-size: 0.82rem;
  font-weight: 650;
  letter-spacing: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.panel-count {
  display: inline-grid;
  min-width: 1.35rem;
  height: 1.35rem;
  border-radius: 4px;
  place-items: center;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
  font-size: 0.68rem;
}

.panel-loading {
  display: grid;
  min-height: 8rem;
  place-items: center;
}

.compact-empty {
  margin: auto;
  padding: 1.5rem 0.75rem;
}

.compact-empty :deep(.arco-empty-image) {
  height: 2.5rem;
}

.compact-empty :deep(.arco-empty-description) {
  color: var(--mcnp-text-faint);
  font-size: 0.75rem;
}

.core-list,
.instance-list {
  min-height: 0;
  overflow: auto;
}

.core-item,
.instance-item {
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  border-radius: 0;
  padding: 0.7rem 0.8rem;
  background: transparent;
  color: var(--mcnp-text);
  cursor: pointer;
  text-align: left;
}

.core-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.65rem;
}

.instance-item {
  display: grid;
  grid-template-columns: 0.25rem minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.65rem;
}

.core-item:hover,
.instance-item:hover {
  background: var(--mcnp-surface-hover);
}

.core-item.selected,
.instance-item.selected {
  box-shadow: inset 3px 0 0 var(--mcnp-primary);
  background: var(--mcnp-primary-soft);
}

.item-copy,
.item-meta {
  display: grid;
  min-width: 0;
}

.item-copy {
  gap: 0.28rem;
}

.item-meta {
  justify-items: end;
  gap: 0.3rem;
}

.item-copy strong,
.item-copy small,
.item-meta small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-copy strong {
  color: var(--mcnp-text);
  font-size: 0.78rem;
  font-weight: 600;
}

.item-copy small,
.item-meta small {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
}

.status {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  gap: 0.35rem;
  color: var(--mcnp-text-muted);
  font-size: 0.64rem;
  font-style: normal;
  font-weight: 600;
  white-space: nowrap;
}

.status > span {
  width: 0.38rem;
  height: 0.38rem;
  border-radius: 50%;
  background: currentColor;
}

.status-online,
.status-running {
  color: var(--mcnp-success);
}

.status-starting,
.status-stopping,
.status-degraded {
  color: var(--mcnp-warning);
}

.status-auth-failed,
.status-incompatible,
.status-failed {
  color: var(--mcnp-danger);
}

.instance-state-mark {
  width: 0.2rem;
  height: 2.1rem;
  border-radius: 2px;
  background: var(--mcnp-text-faint);
}

.instance-state-mark.status-online,
.instance-state-mark.status-running {
  background: var(--mcnp-success);
}

.instance-state-mark.status-starting,
.instance-state-mark.status-stopping,
.instance-state-mark.status-degraded {
  background: var(--mcnp-warning);
}

.instance-state-mark.status-auth-failed,
.instance-state-mark.status-incompatible,
.instance-state-mark.status-failed {
  background: var(--mcnp-danger);
}

.instance-search {
  padding: 0.6rem 0.7rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  background: var(--mcnp-surface-raised);
}

.instance-search :deep(.arco-input-wrapper) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface);
}

.console-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 4rem;
  gap: 1rem;
  padding: 0.65rem 0.9rem 0.65rem 1rem;
  border-bottom: 1px solid var(--mcnp-border);
}

.console-title {
  display: grid;
  min-width: 0;
  gap: 0.3rem;
}

.console-title p {
  overflow: hidden;
  color: var(--mcnp-text-faint);
  font-size: 0.68rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 0.4rem;
}

.start-button {
  background: var(--mcnp-primary);
}

.view-tabs {
  display: flex;
  flex: 0 0 auto;
  gap: 0.25rem;
  padding: 0 0.85rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface-raised);
}

.view-tab {
  display: inline-flex;
  align-items: center;
  min-height: 2.6rem;
  gap: 0.4rem;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  padding: 0 0.7rem;
  background: transparent;
  color: var(--mcnp-text-muted);
  cursor: pointer;
  font-size: 0.74rem;
  font-weight: 600;
}

.view-tab:hover:not(:disabled) {
  color: var(--mcnp-text);
}

.view-tab.selected {
  border-bottom-color: var(--mcnp-primary);
  color: var(--mcnp-primary);
}

.view-tab:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.console-view {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
}

.runtime-strip {
  display: grid;
  flex: 0 0 auto;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface);
}

.runtime-item {
  display: grid;
  min-width: 0;
  gap: 0.3rem;
  padding: 0.65rem 0.8rem;
  border-right: 1px solid var(--mcnp-border-subtle);
}

.runtime-item:last-child {
  border-right: 0;
}

.runtime-item > span {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

.runtime-item strong {
  overflow: hidden;
  color: var(--mcnp-text);
  font-size: 0.7rem;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.form-error,
.notice {
  border-radius: 4px;
  padding: 0.65rem 0.75rem;
  font-size: 0.76rem;
}

.form-error {
  background: var(--mcnp-danger-soft);
  color: var(--mcnp-danger);
}

.notice {
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
}

.workspace-message {
  flex: 0 0 auto;
  margin: 0.6rem 0.75rem 0;
}

.terminal-shell {
  display: flex;
  min-height: 12rem;
  flex: 1;
  flex-direction: column;
  margin: 0.75rem 0.75rem 0;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px 4px 0 0;
  background: var(--mcnp-console);
}

.terminal-toolbar {
  display: flex;
  min-height: 2.25rem;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  padding: 0 0.45rem 0 0.75rem;
  border-bottom: 1px solid #2c2e33;
  background: #18191c;
  color: #b7bbc3;
  font-size: 0.68rem;
}

.terminal-toolbar > span {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.terminal-toolbar small {
  color: #6f747e;
  font-size: 0.62rem;
}

.terminal-refresh {
  color: #8e939c;
}

.log-pane {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 0.75rem;
  color: #c9cdd4;
  font-family: "Cascadia Mono", Consolas, monospace;
}

.log-pane ol {
  display: grid;
  gap: 0.22rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.log-pane li {
  display: grid;
  grid-template-columns: 6.5rem minmax(0, 1fr);
  gap: 0.7rem;
  min-width: 0;
  border-left: 2px solid #484b52;
  padding: 0.22rem 0.45rem;
  color: #c9cdd4;
  font-size: 0.7rem;
  line-height: 1.45;
}

.log-pane time {
  color: #70757f;
}

.log-pane span {
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.log-pane .stream-stderr {
  border-left-color: #d85f5f;
  color: #ef9a9a;
}

.log-pane .stream-system {
  border-left-color: #3c89e8;
}

.terminal-muted {
  color: #747983;
  font-size: 0.72rem;
}

.command-row {
  display: grid;
  flex: 0 0 auto;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.55rem;
  padding: 0.65rem 0.75rem 0.75rem;
  background: var(--mcnp-surface);
}

.command-row :deep(.arco-input-wrapper) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface-raised);
}

.muted {
  color: var(--mcnp-text-faint);
  font-size: 0.76rem;
}

.config-empty {
  padding: 1rem;
}

@media (max-width: 60rem) {
  .workspace {
    overflow: auto;
  }

  .workspace-page {
    height: auto;
    min-height: calc(100vh - 3.5rem);
    overflow: visible;
  }

  .workspace-grid {
    min-height: 52rem;
    flex: 0 0 auto;
    grid-template-columns: minmax(13rem, 0.8fr) minmax(17rem, 1.2fr);
    grid-template-rows: 19rem 34rem;
    overflow: visible;
  }

  .instance-panel {
    border-right: 0;
  }

  .console-panel {
    grid-column: 1 / -1;
    border-top: 1px solid var(--mcnp-border);
  }
}

@media (max-width: 44rem) {
  .topbar-content {
    gap: 0.65rem;
    padding: 0 0.75rem;
  }

  .brand strong,
  .primary-nav,
  .core-health,
  .user-name {
    display: none;
  }

  .topbar-tools {
    justify-self: end;
  }

  .user-identity {
    padding-left: 0.45rem;
  }

  .workspace-page {
    padding: 0.85rem 0.75rem 1rem;
  }

  .page-title-row {
    align-items: flex-start;
    flex-direction: column;
  }

  .workspace-summary {
    justify-content: flex-start;
  }

  .workspace-grid {
    display: flex;
    min-height: 0;
    flex-direction: column;
  }

  .core-panel,
  .instance-panel {
    min-height: 16rem;
    max-height: 20rem;
    border-right: 0;
    border-bottom: 1px solid var(--mcnp-border);
  }

  .console-panel {
    min-height: 38rem;
    border-top: 0;
  }

  .console-head {
    align-items: stretch;
    flex-direction: column;
  }

  .actions {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .runtime-strip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .runtime-item:nth-child(2) {
    border-right: 0;
  }

  .runtime-item:nth-child(-n + 2) {
    border-bottom: 1px solid var(--mcnp-border-subtle);
  }

  .log-pane li {
    grid-template-columns: 1fr;
    gap: 0.15rem;
  }

  .command-row {
    grid-template-columns: 1fr;
  }
}
</style>
