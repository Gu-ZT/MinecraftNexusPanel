<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue';

import { ApiRequestError, createPanelApiClient } from '@mcnp/api-client';
import type { Core, Instance, InstanceState, LogLine, User } from '@mcnp/api-client';
import type { PanelApiClient } from '@mcnp/api-client';
import { McnpEmptyState } from '@mcnp/ui';
import type { DesktopRuntimeInfo } from '@mcnp/platform';

import ConfigEditor from '../components/ConfigEditor.vue';
import { useApplicationStore } from '../stores/application';

const CSRF_STORAGE_KEY = 'mcnp.csrfToken';
const ACCESS_TOKEN_STORAGE_KEY = 'mcnp.accessToken';

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
const loading = ref(false);
const loginPending = ref(false);
const actionPending = ref<string | null>(null);
const errorMessage = ref('');
const noticeMessage = ref('');
const activeView = ref<'console' | 'config'>('console');
const panelApiClient = shallowRef<PanelApiClient>(createPanelApiClient({
  baseUrl: application.platform.apiBaseUrl,
  getAccessToken: () => accessToken.value || undefined,
  getCsrfToken: () => csrfToken.value || undefined,
}));

const selectedCore = computed(() => cores.value.find((core) => core.id === selectedCoreId.value) ?? null);
const selectedInstance = computed(
  () => instances.value.find((instance) => instance.id === selectedInstanceId.value) ?? null,
);
const canStartSelectedInstance = computed(() => canStart(selectedInstance.value?.runtime.state));
const canStopSelectedInstance = computed(() => canStop(selectedInstance.value?.runtime.state));

onMounted(() => {
  void restoreSession();
});

async function restoreSession(): Promise<void> {
  try {
    if (application.platform.initialize) {
      desktopRuntime.value = await application.platform.initialize();
      panelApiClient.value = createPanelApiClient({
        baseUrl: desktopRuntime.value.apiBaseUrl,
        getAccessToken: () => accessToken.value || undefined,
        getCsrfToken: () => csrfToken.value || undefined,
      });
    }
  } catch (error) {
    if (application.platform.kind === 'desktop') {
      errorMessage.value = describeError(error, '无法启动本地 MCNP 服务');
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
        noticeMessage.value = describeError(error, '引导凭据清理失败，请稍后重试');
      }
    }
    try {
      await loadWorkspace();
    } catch (error) {
      errorMessage.value = describeError(error, '登录成功，但加载工作区失败');
    }
  } catch (error) {
    errorMessage.value = describeError(error, '登录失败');
  } finally {
    loginPending.value = false;
  }
}

async function signOut(): Promise<void> {
  actionPending.value = 'logout';
  try {
    await api().logout();
  } catch {
    clearSession();
    return;
  } finally {
    actionPending.value = null;
  }
  clearSession();
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
    if (!cores.value.some((core) => core.id === selectedCoreId.value)) {
      selectedCoreId.value = cores.value[0]?.id ?? '';
    }
    await loadInstances();
  } catch (error) {
    errorMessage.value = describeError(error, '无法加载工作台');
  } finally {
    loading.value = false;
  }
}

async function selectCore(coreId: string): Promise<void> {
  selectedCoreId.value = coreId;
  selectedInstanceId.value = '';
  activeView.value = 'console';
  await loadInstances();
}

async function loadInstances(): Promise<void> {
  logs.value = [];
  instances.value = [];
  if (!selectedCoreId.value) {
    selectedInstanceId.value = '';
    return;
  }

  try {
    const page = await api().listInstances(selectedCoreId.value);
    instances.value = page.items;
    if (!instances.value.some((instance) => instance.id === selectedInstanceId.value)) {
      selectedInstanceId.value = instances.value[0]?.id ?? '';
    }
    await loadLogs();
  } catch (error) {
    selectedInstanceId.value = '';
    errorMessage.value = describeError(error, '无法加载实例');
  }
}

async function selectInstance(instanceId: string): Promise<void> {
  selectedInstanceId.value = instanceId;
  activeView.value = 'console';
  await loadLogs();
}

async function loadLogs(): Promise<void> {
  if (!selectedCoreId.value || !selectedInstanceId.value) {
    logs.value = [];
    return;
  }

  try {
    const page = await api().getInstanceLogs(selectedCoreId.value, selectedInstanceId.value);
    logs.value = page.items;
  } catch (error) {
    logs.value = [];
    errorMessage.value = describeError(error, '无法读取控制台');
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
  try {
    await api().sendInstanceCommand(selectedCoreId.value, selectedInstanceId.value, trimmedCommand);
    command.value = '';
    noticeMessage.value = '命令已发送';
    await loadLogs();
  } catch (error) {
    errorMessage.value = describeError(error, '命令发送失败');
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
  try {
    await operation();
    noticeMessage.value = actionNotice(action);
    await loadInstances();
  } catch (error) {
    errorMessage.value = describeError(error, '实例操作失败');
  } finally {
    actionPending.value = null;
  }
}

function api() {
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
  selectedCoreId.value = '';
  selectedInstanceId.value = '';
  activeView.value = 'console';
}

function describeError(error: unknown, fallback: string): string {
  if (error instanceof ApiRequestError) {
    return `${fallback}：${error.message}`;
  }
  if (error instanceof Error) {
    return `${fallback}：${error.message}`;
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
    return '启动任务已提交';
  }
  if (action === 'stop') {
    return '停止任务已提交';
  }
  if (action === 'kill') {
    return '强制终止任务已提交';
  }

  return '操作已提交';
}

function formatDate(value: string | null | undefined): string {
  if (!value) {
    return '未记录';
  }

  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    month: '2-digit',
    day: '2-digit',
  }).format(new Date(value));
}

function statusClass(status: string): string {
  return `status status-${status.toLowerCase().replace('_', '-')}`;
}
</script>

<template>
  <form v-if="!currentUser" class="login-shell" @submit.prevent="signIn">
    <section class="login-panel">
      <div>
        <p class="eyebrow">MCNP Panel</p>
        <h1>登录控制台</h1>
      </div>
      <div v-if="desktopRuntime?.initialAdminPassword" class="bootstrap-credentials">
        <strong>首次启动管理员凭据</strong>
        <p>请先使用以下凭据登录，成功后引导密码会从本机配置中删除。</p>
        <code>用户名：{{ desktopRuntime.initialAdminUsername }}</code>
        <code>密码：{{ desktopRuntime.initialAdminPassword }}</code>
      </div>
      <label>
        <span>用户名</span>
        <input v-model="username" autocomplete="username" required />
      </label>
      <label>
        <span>密码</span>
        <input v-model="password" autocomplete="current-password" required type="password" />
      </label>
      <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
      <button class="primary-button" :disabled="loginPending || !username || !password" type="submit">
        {{ loginPending ? '正在登录' : '登录' }}
      </button>
    </section>
  </form>

  <div v-else class="workspace">
    <header class="topbar">
      <div class="brand">
        <strong>MCNP</strong>
        <span>实例控制台</span>
      </div>
      <div class="session">
        <span>{{ currentUser.displayName }}</span>
        <button type="button" @click="refreshWorkspace">刷新</button>
        <button type="button" :disabled="actionPending === 'logout'" @click="signOut">退出</button>
      </div>
    </header>

    <main class="workspace-grid">
      <aside class="core-panel" aria-label="Core 列表">
        <div class="panel-heading">
          <h2>Core</h2>
          <span>{{ cores.length }}</span>
        </div>
        <McnpEmptyState v-if="!loading && cores.length === 0" title="尚未连接 Core" />
        <div v-else class="core-list">
          <button
            v-for="core in cores"
            :key="core.id"
            class="core-item"
            :class="{ selected: core.id === selectedCoreId }"
            type="button"
            @click="selectCore(core.id)"
          >
            <span>
              <strong>{{ core.name }}</strong>
              <small>{{ core.address }}</small>
            </span>
            <i :class="statusClass(core.status)">{{ core.status }}</i>
          </button>
        </div>
      </aside>

      <section class="instance-panel" aria-label="实例列表">
        <div class="panel-heading">
          <h2>实例</h2>
          <span>{{ selectedCore?.name ?? '未选择 Core' }}</span>
        </div>
        <McnpEmptyState v-if="!selectedCoreId" title="请选择 Core" />
        <McnpEmptyState v-else-if="instances.length === 0" title="暂无实例" />
        <div v-else class="instance-list">
          <button
            v-for="instance in instances"
            :key="instance.id"
            class="instance-item"
            :class="{ selected: instance.id === selectedInstanceId }"
            type="button"
            @click="selectInstance(instance.id)"
          >
            <span>
              <strong>{{ instance.name }}</strong>
              <small>{{ instance.id }} · {{ instance.kind }}</small>
            </span>
            <i :class="statusClass(instance.runtime.state)">{{ instance.runtime.state }}</i>
          </button>
        </div>
      </section>

      <section class="console-panel" aria-label="控制台">
        <div class="console-head">
          <div>
            <h2>{{ selectedInstance?.name ?? '控制台' }}</h2>
            <p v-if="selectedInstance">
              {{ selectedInstance.directory }} · PID {{ selectedInstance.runtime.pid ?? '未运行' }}
            </p>
          </div>
          <div class="actions">
            <button
              type="button"
              :disabled="!selectedInstance || !canStartSelectedInstance || actionPending !== null"
              @click="startInstance"
            >
              启动
            </button>
            <button
              type="button"
              :disabled="!selectedInstance || !canStopSelectedInstance || actionPending !== null"
              @click="stopInstance"
            >
              停止
            </button>
            <button type="button" :disabled="!selectedInstance || actionPending !== null" @click="killInstance">
              终止
            </button>
          </div>
        </div>

        <div class="view-tabs" role="tablist" aria-label="实例视图">
          <button
            class="view-tab"
            :class="{ selected: activeView === 'console' }"
            type="button"
            role="tab"
            :aria-selected="activeView === 'console'"
            @click="activeView = 'console'"
          >
            控制台
          </button>
          <button
            class="view-tab"
            :class="{ selected: activeView === 'config' }"
            type="button"
            role="tab"
            :aria-selected="activeView === 'config'"
            :disabled="!selectedInstance"
            @click="activeView = 'config'"
          >
            配置
          </button>
        </div>

        <div v-if="activeView === 'console'" class="console-view">
          <div v-if="selectedInstance" class="runtime-strip">
            <span :class="statusClass(selectedInstance.runtime.state)">{{ selectedInstance.runtime.state }}</span>
            <span>启动：{{ formatDate(selectedInstance.runtime.startedAt) }}</span>
            <span>退出码：{{ selectedInstance.runtime.exitCode ?? '无' }}</span>
          </div>

          <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
          <p v-else-if="noticeMessage" class="notice">{{ noticeMessage }}</p>

          <div class="log-pane">
            <p v-if="!selectedInstance" class="muted">选择实例后显示控制台输出。</p>
            <p v-else-if="logs.length === 0" class="muted">暂无控制台输出。</p>
            <ol v-else>
              <li v-for="line in logs" :key="line.cursor" :class="`stream-${line.stream}`">
                <time>{{ formatDate(line.occurredAt) }}</time>
                <span>{{ line.line }}</span>
              </li>
            </ol>
          </div>

          <form class="command-row" @submit.prevent="sendCommand">
            <input v-model="command" :disabled="!selectedInstance || actionPending !== null" placeholder="输入控制台命令" />
            <button type="submit" :disabled="!selectedInstance || !command.trim() || actionPending !== null">发送</button>
          </form>
        </div>

        <ConfigEditor
          v-else-if="selectedInstance"
          :client="panelApiClient"
          :core-id="selectedCoreId"
          :instance-id="selectedInstance.id"
        />
        <p v-else class="muted config-empty">选择实例后编辑配置。</p>
      </section>
    </main>
  </div>
</template>

<style scoped>
.login-shell {
  display: grid;
  min-height: 100vh;
  place-items: center;
  padding: 1.5rem;
  background:
    linear-gradient(135deg, rgba(32, 107, 58, 0.12), transparent 42%),
    linear-gradient(315deg, rgba(46, 86, 150, 0.14), transparent 38%),
    #f4f6f4;
}

.login-panel {
  display: grid;
  width: min(100%, 24rem);
  gap: 1rem;
  padding: 1.5rem;
  border: 1px solid #d7dcd8;
  border-radius: 8px;
  background: #ffffff;
  box-shadow: 0 18px 45px rgba(24, 32, 27, 0.08);
}

.bootstrap-credentials {
  display: grid;
  gap: 0.35rem;
  border-left: 3px solid #2f7d4a;
  padding: 0.7rem 0.8rem;
  background: #eef7f1;
  color: #31523d;
  font-size: 0.78rem;
}

.bootstrap-credentials p {
  line-height: 1.4;
}

.bootstrap-credentials code {
  overflow-wrap: anywhere;
  color: #1f6239;
  font-family: "Cascadia Mono", Consolas, monospace;
}

.eyebrow {
  margin: 0 0 0.35rem;
  color: #2f6f95;
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
}

h1,
h2,
p {
  margin: 0;
}

h1 {
  color: #18201b;
  font-size: 1.65rem;
  letter-spacing: 0;
}

label {
  display: grid;
  gap: 0.45rem;
  color: #415047;
  font-size: 0.86rem;
  font-weight: 650;
}

input {
  width: 100%;
  min-height: 2.55rem;
  border: 1px solid #c8d0cb;
  border-radius: 6px;
  padding: 0 0.75rem;
  background: #ffffff;
  color: #18201b;
}

button {
  min-height: 2.35rem;
  border: 1px solid #c7d0ca;
  border-radius: 6px;
  padding: 0 0.8rem;
  background: #ffffff;
  color: #26352b;
  cursor: pointer;
  font-weight: 650;
}

button:hover:not(:disabled) {
  border-color: #2f7d4a;
  color: #1f6239;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.primary-button {
  border-color: #206b3a;
  background: #206b3a;
  color: #ffffff;
}

.workspace {
  min-height: 100vh;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 3.5rem;
  gap: 1rem;
  padding: 0 1.25rem;
  border-bottom: 1px solid #d7dcd8;
  background: #ffffff;
}

.brand,
.session,
.actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.brand strong {
  color: #206b3a;
  font-size: 1rem;
  letter-spacing: 0;
}

.brand span,
.session span {
  color: #637068;
  font-size: 0.875rem;
}

.workspace-grid {
  display: grid;
  grid-template-columns: minmax(15rem, 18rem) minmax(17rem, 22rem) minmax(0, 1fr);
  min-height: calc(100vh - 3.5rem);
}

.core-panel,
.instance-panel,
.console-panel {
  min-width: 0;
  border-right: 1px solid #d7dcd8;
  background: #fbfcfb;
}

.console-panel {
  display: flex;
  min-height: 0;
  flex-direction: column;
  border-right: 0;
  background: #f7f8f7;
}

.view-tabs {
  display: flex;
  gap: 0.35rem;
  padding: 0.55rem 1rem 0;
  border-bottom: 1px solid #d7dcd8;
  background: #ffffff;
}

.view-tab {
  min-height: 2.15rem;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  padding: 0 0.65rem;
  background: transparent;
  color: #637068;
  cursor: pointer;
  font-size: 0.8rem;
  font-weight: 700;
}

.view-tab.selected {
  border-bottom-color: #206b3a;
  color: #1f6239;
}

.view-tab:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.console-view {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
}

.panel-heading,
.console-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  min-height: 4rem;
  padding: 0.95rem 1rem;
  border-bottom: 1px solid #d7dcd8;
}

.panel-heading h2,
.console-head h2 {
  color: #18201b;
  font-size: 0.95rem;
  letter-spacing: 0;
}

.panel-heading span,
.console-head p,
.muted {
  color: #637068;
  font-size: 0.82rem;
}

.core-list,
.instance-list {
  display: grid;
  gap: 0.5rem;
  padding: 0.75rem;
}

.core-item,
.instance-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  min-height: 4rem;
  padding: 0.75rem;
  text-align: left;
}

.core-item.selected,
.instance-item.selected {
  border-color: #2f7d4a;
  background: #eef7f1;
}

.core-item span,
.instance-item span {
  display: grid;
  min-width: 0;
  gap: 0.3rem;
}

.core-item strong,
.instance-item strong {
  overflow: hidden;
  color: #18201b;
  font-size: 0.92rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.core-item small,
.instance-item small {
  overflow: hidden;
  color: #637068;
  font-size: 0.78rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status {
  display: inline-flex;
  align-items: center;
  min-height: 1.45rem;
  border-radius: 999px;
  padding: 0 0.55rem;
  background: #ecefed;
  color: #4d5d54;
  font-size: 0.72rem;
  font-style: normal;
  font-weight: 750;
}

.status-online,
.status-running {
  background: #dff2e6;
  color: #1d6b3b;
}

.status-starting,
.status-stopping,
.status-degraded {
  background: #fff3cf;
  color: #8a5a00;
}

.status-offline,
.status-stopped,
.status-created,
.status-unknown {
  background: #e8edf4;
  color: #3d536e;
}

.status-auth-failed,
.status-incompatible,
.status-failed {
  background: #fde4e2;
  color: #9a2f29;
}

.runtime-strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.65rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #d7dcd8;
  background: #ffffff;
  color: #4b5a51;
  font-size: 0.82rem;
}

.form-error,
.notice {
  margin: 0.75rem 1rem 0;
  border-radius: 6px;
  padding: 0.65rem 0.75rem;
  font-size: 0.86rem;
}

.form-error {
  background: #fde4e2;
  color: #8f2b25;
}

.notice {
  background: #e1f2ea;
  color: #1f6239;
}

.log-pane {
  min-height: 0;
  overflow: auto;
  padding: 1rem;
  font-family: "Cascadia Mono", Consolas, monospace;
}

.log-pane ol {
  display: grid;
  gap: 0.35rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.log-pane li {
  display: grid;
  grid-template-columns: 7rem minmax(0, 1fr);
  gap: 0.75rem;
  min-width: 0;
  border-left: 3px solid #c7d0ca;
  padding: 0.35rem 0.5rem;
  background: #ffffff;
  color: #253229;
  font-size: 0.8rem;
  line-height: 1.45;
}

.log-pane time {
  color: #6b786f;
}

.log-pane span {
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.stream-stderr {
  border-left-color: #c64d43;
}

.stream-system {
  border-left-color: #2f6f95;
}

.command-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.65rem;
  padding: 0.85rem 1rem;
  border-top: 1px solid #d7dcd8;
  background: #ffffff;
}

.config-empty {
  padding: 1rem;
}

@media (max-width: 62rem) {
  .workspace-grid {
    grid-template-columns: 1fr;
  }

  .core-panel,
  .instance-panel,
  .console-panel {
    border-right: 0;
    border-bottom: 1px solid #d7dcd8;
  }
}

@media (max-width: 42rem) {
  .topbar,
  .console-head {
    align-items: stretch;
    flex-direction: column;
    padding: 0.85rem 1rem;
  }

  .session,
  .actions,
  .command-row {
    grid-template-columns: 1fr;
    width: 100%;
  }

  .session,
  .actions {
    display: grid;
  }

  .log-pane li {
    grid-template-columns: 1fr;
  }
}
</style>
