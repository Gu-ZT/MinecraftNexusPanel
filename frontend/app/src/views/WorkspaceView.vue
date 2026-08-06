<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  InputPassword as AInputPassword,
} from '@arco-design/web-vue';
import { IconLock, IconUser } from '@arco-design/web-vue/es/icon';
import { computed, onMounted, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';

import { createPanelApiClient } from '@mcnp/api-client';
import type {
  Core,
  Instance,
  PanelApiClient,
  PanelAuditEvent,
  SessionTokens,
  User,
} from '@mcnp/api-client';
import type { DesktopRuntimeInfo } from '@mcnp/platform';

import ControlPanelHeader from '../components/ControlPanelHeader.vue';
import DashboardView from '../components/DashboardView.vue';
import InstanceListView from '../components/InstanceListView.vue';
import InstanceWorkspace from '../components/InstanceWorkspace.vue';
import LocalSettingsView from '../components/LocalSettingsView.vue';
import NodeListView from '../components/NodeListView.vue';
import PreferenceControls from '../components/PreferenceControls.vue';
import { useApplicationStore } from '../stores/application';
import { describeError } from '../utils/presentation';

type LifecycleAction = 'start' | 'stop' | 'kill' | 'reset';
type WorkspaceViewName = 'overview' | 'console' | 'config' | 'files';

const CSRF_STORAGE_KEY = 'mcnp.csrfToken';
const ACCESS_TOKEN_STORAGE_KEY = 'mcnp.accessToken';
const NATIVE_REFRESH_SKEW_MS = 60_000;
const MAX_TIMER_DELAY_MS = 2_147_483_647;

let nativeRefreshTimer: ReturnType<typeof setTimeout> | null = null;
let nativeRefreshInFlight: Promise<boolean> | null = null;

const { t } = useI18n();
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
const instances = ref<Instance[]>([]);
const auditEvents = ref<PanelAuditEvent[]>([]);
const loading = ref(false);
const loginPending = ref(false);
const actionPending = ref<string | null>(null);
const autostartEnabled = ref<boolean | null>(null);
const autostartPending = ref(false);
const logsPending = ref(false);
const errorMessage = ref('');
const noticeMessage = ref('');
const panelApiClient = shallowRef<PanelApiClient>(createClient(application.platform.apiBaseUrl));

const selectedCore = computed(() => {
  const coreId = routeParam('coreId');
  return cores.value.find((core) => core.id === coreId) ?? null;
});
const selectedInstance = computed(() => {
  const coreId = routeParam('coreId');
  const instanceId = routeParam('instanceId');
  return instances.value.find((instance) => instance.coreId === coreId && instance.id === instanceId) ?? null;
});
const activeInstanceView = computed<WorkspaceViewName>(() => {
  const value = routeParam('view');
  return value === 'console' || value === 'config' || value === 'files' ? value : 'overview';
});
const effectiveApiBaseUrl = computed(
  () => desktopRuntime.value?.apiBaseUrl ?? application.platform.apiBaseUrl,
);

onMounted(() => {
  void restoreSession();
});

async function restoreSession(): Promise<void> {
  try {
    if (application.platform.initialize) {
      desktopRuntime.value = await application.platform.initialize();
      panelApiClient.value = createClient(desktopRuntime.value.apiBaseUrl);
      if (!username.value && desktopRuntime.value.initialAdminUsername) {
        username.value = desktopRuntime.value.initialAdminUsername;
      }
      await loadAutostartStatus();
      await restoreDesktopSession();
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
  let credentialStoreFailed = false;
  try {
    const clientType = application.platform.kind === 'desktop' ? 'NATIVE' : 'BROWSER';
    const response = await api().login(username.value.trim(), password.value, clientType);
    currentUser.value = response.user;
    applySession(response.session);
    storeSession();
    password.value = '';

    try {
      await persistDesktopRefreshToken(response.session.refreshToken);
    } catch (error) {
      credentialStoreFailed = true;
      try {
        await clearDesktopRefreshToken();
      } catch {
        // 原错误已经用于提示用户；这里仅尽力清除可能属于上一会话的旧令牌。
      }
      throw error;
    }

    if (desktopRuntime.value?.initialAdminPassword && application.platform.completeInitialAdmin) {
      try {
        await application.platform.completeInitialAdmin();
        desktopRuntime.value = { ...desktopRuntime.value, initialAdminPassword: null };
      } catch (error) {
        noticeMessage.value = describeError(error, t('error.bootstrapCleanup'));
      }
    }

    await loadWorkspace();
  } catch (error) {
    const fallback = credentialStoreFailed ? t('error.refreshTokenStore') : t('error.login');
    clearSession();
    errorMessage.value = describeError(error, fallback);
  } finally {
    loginPending.value = false;
  }
}

async function signOut(): Promise<void> {
  actionPending.value = 'logout';
  let refreshTokenError: unknown;
  clearNativeRefreshTimer();
  if (nativeRefreshInFlight) {
    await nativeRefreshInFlight;
  }
  try {
    await api().logout();
  } catch {
    // Panel 失联时仍清理本机会话，避免用户无法退出当前设备。
  } finally {
    try {
      await clearDesktopRefreshToken();
    } catch (error) {
      refreshTokenError = error;
    }
    actionPending.value = null;
    clearSession();
    await router.replace({ name: 'dashboard' });
  }
  if (refreshTokenError) {
    errorMessage.value = describeError(refreshTokenError, t('error.refreshTokenClear'));
  }
}

async function loadWorkspace(): Promise<void> {
  loading.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const [corePage, auditPage] = await Promise.all([
      api().listCores(),
      api().listAuditEvents(30).catch(() => null),
    ]);
    cores.value = corePage.items;
    auditEvents.value = auditPage?.items ?? [];

    const instancePages = await Promise.allSettled(
      cores.value.map(async (core) => ({ coreId: core.id, page: await api().listInstances(core.id) })),
    );
    instances.value = instancePages.flatMap((result) => (result.status === 'fulfilled' ? result.value.page.items : []));
    const failedCoreCount = instancePages.filter((result) => result.status === 'rejected').length;
    if (failedCoreCount > 0) {
      noticeMessage.value = t('notice.partialInstances', { count: failedCoreCount });
    }
  } catch (error) {
    errorMessage.value = describeError(error, t('error.workspace'));
  } finally {
    loading.value = false;
  }
}

async function runLifecycleAction(
  action: LifecycleAction,
  coreId: string,
  instanceId: string,
): Promise<void> {
  actionPending.value = `${action}:${coreId}:${instanceId}`;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    if (action === 'start') {
      await api().startInstance(coreId, instanceId);
    } else if (action === 'stop') {
      await api().stopInstance(coreId, instanceId);
    } else if (action === 'kill') {
      await api().killInstance(coreId, instanceId);
    } else {
      await api().resetInstance(coreId, instanceId);
    }
    noticeMessage.value = t(`notice.${action}Submitted`);
    await refreshCoreInstances(coreId);
  } catch (error) {
    errorMessage.value = describeError(error, t('error.instanceAction'));
  } finally {
    actionPending.value = null;
  }
}

async function loadAutostartStatus(): Promise<void> {
  if (!application.platform.isAutostartEnabled) {
    return;
  }
  try {
    autostartEnabled.value = await application.platform.isAutostartEnabled();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.autostartRead'));
  }
}

async function restoreDesktopSession(): Promise<void> {
  if (application.platform.kind !== 'desktop' || !application.platform.getRefreshToken) {
    return;
  }
  await refreshDesktopSession();
}

function refreshDesktopSession(required = false): Promise<boolean> {
  if (nativeRefreshInFlight) {
    return nativeRefreshInFlight;
  }
  const refresh = runDesktopSessionRefresh(required);
  nativeRefreshInFlight = refresh;
  const releaseRefreshLock = () => {
    if (nativeRefreshInFlight === refresh) {
      nativeRefreshInFlight = null;
    }
  };
  void refresh.then(releaseRefreshLock, releaseRefreshLock);
  return refresh;
}

async function runDesktopSessionRefresh(required: boolean): Promise<boolean> {
  if (application.platform.kind !== 'desktop' || !application.platform.getRefreshToken) {
    return false;
  }
  let refreshToken: string | null;
  try {
    refreshToken = await application.platform.getRefreshToken();
  } catch (error) {
    if (required) {
      clearSession();
      errorMessage.value = describeError(error, t('error.sessionRefresh'));
    }
    return false;
  }
  if (!refreshToken) {
    if (required) {
      clearSession();
      errorMessage.value = t('error.sessionRefresh');
    }
    return false;
  }

  try {
    const session = await api().refreshNative(refreshToken);
    applySession(session);
    await persistDesktopRefreshToken(session.refreshToken);
    storeSession();
    return true;
  } catch (error) {
    try {
      await api().logout();
    } catch {
      // Panel 失联或会话已失效时，继续完成本地安全存储清理。
    }
    try {
      await clearDesktopRefreshToken();
    } catch {
      // 令牌已经失效时，清理失败也不能阻止本地会话回到登录页。
    }
    clearSession();
    errorMessage.value = describeError(error, t('error.sessionRefresh'));
    return false;
  }
}

function applySession(session: SessionTokens): void {
  accessToken.value = session.accessToken ?? '';
  csrfToken.value = session.csrfToken ?? '';
  if (application.platform.kind === 'desktop' && session.accessToken) {
    scheduleNativeRefresh(session.accessExpiresAt);
  }
}

function scheduleNativeRefresh(accessExpiresAt: string): void {
  clearNativeRefreshTimer();
  const expirationTimestamp = Date.parse(accessExpiresAt);
  if (!Number.isFinite(expirationTimestamp)) {
    return;
  }

  // 浏览器休眠会延迟 timer；恢复后延迟值已经到期，回调会尽快执行一次安全轮换。
  const requestedDelay = Math.max(0, expirationTimestamp - Date.now() - NATIVE_REFRESH_SKEW_MS);
  nativeRefreshTimer = setTimeout(
    () => void refreshDesktopSession(true),
    Math.min(requestedDelay, MAX_TIMER_DELAY_MS),
  );
}

function clearNativeRefreshTimer(): void {
  if (nativeRefreshTimer !== null) {
    clearTimeout(nativeRefreshTimer);
    nativeRefreshTimer = null;
  }
}

async function persistDesktopRefreshToken(refreshToken: string | null): Promise<void> {
  if (refreshToken && application.platform.setRefreshToken) {
    await application.platform.setRefreshToken(refreshToken);
  } else if (!refreshToken && application.platform.clearRefreshToken) {
    await application.platform.clearRefreshToken();
  }
}

async function clearDesktopRefreshToken(): Promise<void> {
  if (application.platform.clearRefreshToken) {
    await application.platform.clearRefreshToken();
  }
}

async function changeAutostart(enabled: boolean): Promise<void> {
  if (!application.platform.setAutostartEnabled) {
    return;
  }
  autostartPending.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    autostartEnabled.value = await application.platform.setAutostartEnabled(enabled);
    noticeMessage.value = autostartEnabled.value ? t('notice.autostartEnabled') : t('notice.autostartDisabled');
  } catch (error) {
    errorMessage.value = describeError(error, t('error.autostartUpdate'));
  } finally {
    autostartPending.value = false;
  }
}

async function openLogs(): Promise<void> {
  if (!application.platform.openLogDirectory) {
    return;
  }
  logsPending.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    await application.platform.openLogDirectory();
    noticeMessage.value = t('notice.logsOpened');
  } catch (error) {
    errorMessage.value = describeError(error, t('error.logsOpen'));
  } finally {
    logsPending.value = false;
  }
}

async function refreshCoreInstances(coreId: string): Promise<void> {
  const page = await api().listInstances(coreId);
  instances.value = [...instances.value.filter((instance) => instance.coreId !== coreId), ...page.items];
}

function createClient(baseUrl: string): PanelApiClient {
  return createPanelApiClient({
    baseUrl,
    getAccessToken: () => accessToken.value || undefined,
    getCsrfToken: () => csrfToken.value || undefined,
  });
}

function api(): PanelApiClient {
  return panelApiClient.value;
}

function routeParam(name: string): string {
  const value = route.params[name];
  return typeof value === 'string' ? value : '';
}

function storeSession(): void {
  if (accessToken.value) {
    sessionStorage.setItem(ACCESS_TOKEN_STORAGE_KEY, accessToken.value);
  } else {
    sessionStorage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
  }
  sessionStorage.setItem(CSRF_STORAGE_KEY, csrfToken.value);
}

function clearSession(): void {
  clearNativeRefreshTimer();
  sessionStorage.removeItem(CSRF_STORAGE_KEY);
  sessionStorage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
  csrfToken.value = '';
  accessToken.value = '';
  currentUser.value = null;
  cores.value = [];
  instances.value = [];
  auditEvents.value = [];
}
</script>

<template>
  <form v-if="!currentUser" class="login-shell" @submit.prevent="signIn">
    <div class="login-utilities"><PreferenceControls /></div>
    <section class="login-panel">
      <div class="login-brand">
        <span class="brand-mark" aria-hidden="true">MN</span>
        <span>{{ t('app.name') }}</span>
      </div>
      <div class="login-heading">
        <p>{{ t('auth.eyebrow') }}</p>
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
        <a-input-password v-model="password" autocomplete="current-password" size="large" allow-clear>
          <template #prefix><IconLock /></template>
        </a-input-password>
      </label>
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
      <a-button type="primary" size="large" long html-type="submit" :loading="loginPending" :disabled="!username || !password">
        {{ loginPending ? t('auth.loggingIn') : t('auth.login') }}
      </a-button>
    </section>
  </form>

  <div v-else class="control-shell">
    <ControlPanelHeader
      :user="currentUser"
      :cores="cores"
      :loading="loading"
      :signing-out="actionPending === 'logout'"
      @refresh="loadWorkspace"
      @sign-out="signOut"
    />

    <div v-if="errorMessage || noticeMessage" class="global-feedback">
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
      <p v-else class="notice" role="status">{{ noticeMessage }}</p>
    </div>

    <DashboardView
      v-if="route.name === 'dashboard'"
      :cores="cores"
      :instances="instances"
      :audit-events="auditEvents"
      :loading="loading"
    />
    <InstanceListView
      v-else-if="route.name === 'instances' || route.name === 'core-instances'"
      :cores="cores"
      :instances="instances"
      :loading="loading"
      :action-pending="actionPending"
      @action="runLifecycleAction"
    />
    <NodeListView
      v-else-if="route.name === 'nodes'"
      :client="panelApiClient"
      :cores="cores"
      :loading="loading"
    />
    <LocalSettingsView
      v-else-if="route.name === 'settings'"
      :platform-kind="application.platform.kind"
      :api-base-url="effectiveApiBaseUrl"
      :autostart-enabled="autostartEnabled"
      :autostart-pending="autostartPending"
      :logs-pending="logsPending"
      @change-autostart="changeAutostart"
      @open-logs="openLogs"
    />
    <InstanceWorkspace
      v-else-if="selectedCore && selectedInstance"
      :client="panelApiClient"
      :core="selectedCore"
      :instance="selectedInstance"
      :active-view="activeInstanceView"
      :action-pending="actionPending"
      @action="runLifecycleAction"
    />
    <main v-else class="console-page"><a-empty :description="t('instances.notFound')" /></main>
  </div>
</template>

<style scoped>
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
  gap: 1.05rem;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  padding: 1.75rem;
  background: var(--mcnp-surface);
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.12);
}

.login-brand {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  color: var(--mcnp-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
}

.brand-mark {
  display: inline-grid;
  width: 2rem;
  height: 2rem;
  border-radius: 4px;
  place-items: center;
  background: var(--mcnp-primary);
  color: #fff;
  font-size: 0.7rem;
  font-weight: 800;
}

.login-heading {
  display: grid;
  gap: 0.35rem;
  padding-top: 0.4rem;
}

.login-heading p,
.login-heading h1,
.bootstrap-credentials p {
  margin: 0;
}

.login-heading p {
  color: var(--mcnp-primary);
  font-size: 0.7rem;
  font-weight: 700;
}

.login-heading h1 {
  color: var(--mcnp-text);
  font-size: 1.45rem;
}

.bootstrap-credentials {
  display: grid;
  gap: 0.4rem;
  border-left: 3px solid var(--mcnp-primary);
  padding: 0.75rem 0.85rem;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-text-muted);
  font-size: 0.75rem;
}

.bootstrap-credentials code {
  overflow-wrap: anywhere;
  color: var(--mcnp-primary);
}

.login-field {
  display: grid;
  gap: 0.45rem;
  color: var(--mcnp-text-muted);
  font-size: 0.8rem;
  font-weight: 600;
}

.control-shell {
  min-height: 100vh;
  background: var(--mcnp-bg);
  color: var(--mcnp-text);
}

.global-feedback {
  width: min(calc(100% - 3rem), 105rem);
  margin: 0.75rem auto -0.15rem;
}

.global-feedback p {
  margin: 0;
}

@media (max-width: 44rem) {
  .global-feedback {
    width: calc(100% - 1.5rem);
  }
}
</style>
