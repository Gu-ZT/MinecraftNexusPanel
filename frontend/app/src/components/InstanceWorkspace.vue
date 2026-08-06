<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  Popconfirm as APopconfirm,
  Spin as ASpin,
  Tooltip as ATooltip,
} from '@arco-design/web-vue';
import {
  IconApps,
  IconCode,
  IconCommand,
  IconDashboard,
  IconFile,
  IconPlayArrow,
  IconRefresh,
  IconRight,
  IconSend,
  IconSettings,
  IconStop,
} from '@arco-design/web-vue/es/icon';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import type { Core, Instance, LogLine, PanelApiClient } from '@mcnp/api-client';

import ConfigEditor from './ConfigEditor.vue';
import InstanceFileManager from './InstanceFileManager.vue';
import {
  canStartInstance,
  canStopInstance,
  describeError,
  formatDate,
  statusClass,
} from '../utils/presentation';

type InstanceWorkspaceView = 'overview' | 'console' | 'config' | 'files';

const props = defineProps<{
  client: PanelApiClient;
  core: Core;
  instance: Instance;
  activeView: InstanceWorkspaceView;
  actionPending: string | null;
}>();

const emit = defineEmits<{
  action: [action: 'start' | 'stop' | 'kill' | 'reset', coreId: string, instanceId: string];
}>();

const { locale, t, te } = useI18n();
const logs = ref<LogLine[]>([]);
const logsLoading = ref(false);
const command = ref('');
const commandPending = ref(false);
const errorMessage = ref('');
const noticeMessage = ref('');
let logTimer: number | undefined;

const actionKeyPrefix = computed(() => `${props.instance.coreId}:${props.instance.id}`);
const canReset = computed(
  () => props.instance.runtime.state === 'FAILED' || props.instance.runtime.state === 'UNKNOWN',
);

onMounted(() => {
  logTimer = window.setInterval(() => {
    if (props.activeView === 'console' && props.instance.runtime.state === 'RUNNING' && !logsLoading.value) {
      void loadLogs(false);
    }
  }, 3000);
});

onUnmounted(() => {
  if (logTimer !== undefined) {
    window.clearInterval(logTimer);
  }
});

watch(
  () => [props.instance.coreId, props.instance.id, props.activeView],
  () => {
    errorMessage.value = '';
    noticeMessage.value = '';
    if (props.activeView === 'console') {
      void loadLogs(true);
    }
  },
  { immediate: true },
);

async function loadLogs(showLoading: boolean): Promise<void> {
  if (showLoading) {
    logsLoading.value = true;
  }
  try {
    const page = await props.client.getInstanceLogs(props.core.id, props.instance.id);
    logs.value = page.items;
  } catch (error) {
    errorMessage.value = describeError(error, t('error.logs'));
  } finally {
    logsLoading.value = false;
  }
}

async function sendCommand(): Promise<void> {
  const trimmedCommand = command.value.trim();
  if (!trimmedCommand) {
    return;
  }
  commandPending.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    await props.client.sendInstanceCommand(props.core.id, props.instance.id, trimmedCommand);
    command.value = '';
    noticeMessage.value = t('notice.commandSent');
    await loadLogs(false);
  } catch (error) {
    errorMessage.value = describeError(error, t('error.command'));
  } finally {
    commandPending.value = false;
  }
}

function actionKey(action: string): string {
  return `${action}:${actionKeyPrefix.value}`;
}

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}
</script>

<template>
  <main class="instance-workspace-page">
    <nav class="workspace-breadcrumbs" :aria-label="t('nav.breadcrumb')">
      <RouterLink :to="{ name: 'instances' }"><IconApps /> {{ t('nav.instances') }}</RouterLink>
      <IconRight />
      <RouterLink :to="{ name: 'core-instances', params: { coreId: core.id } }">{{ core.name }}</RouterLink>
      <IconRight />
      <strong>{{ instance.name }}</strong>
    </nav>

    <header class="instance-heading">
      <div>
        <span :class="['instance-heading__mark', statusClass(instance.runtime.state)]"></span>
        <div>
          <h1>{{ instance.name }}</h1>
          <p>{{ instance.kind }} · {{ instance.id }} · {{ core.name }}</p>
        </div>
      </div>
      <div class="lifecycle-actions">
        <a-button
          type="primary"
          size="small"
          :loading="actionPending === actionKey('start')"
          :disabled="actionPending !== null || !canStartInstance(instance.runtime.state)"
          @click="emit('action', 'start', core.id, instance.id)"
        >
          <template #icon><IconPlayArrow /></template>
          {{ t('console.start') }}
        </a-button>
        <a-button
          size="small"
          :loading="actionPending === actionKey('stop')"
          :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
          @click="emit('action', 'stop', core.id, instance.id)"
        >
          <template #icon><IconStop /></template>
          {{ t('console.stop') }}
        </a-button>
        <a-popconfirm :content="t('instances.killConfirm', { name: instance.name })" @ok="emit('action', 'kill', core.id, instance.id)">
          <a-button
            status="danger"
            size="small"
            :loading="actionPending === actionKey('kill')"
            :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
          >
            <template #icon><IconStop /></template>
            {{ t('console.kill') }}
          </a-button>
        </a-popconfirm>
        <a-popconfirm :content="t('instances.resetConfirm', { name: instance.name })" @ok="emit('action', 'reset', core.id, instance.id)">
          <a-tooltip :content="t('instances.reset')">
            <a-button
              size="small"
              :loading="actionPending === actionKey('reset')"
              :disabled="actionPending !== null || !canReset"
              :aria-label="t('instances.reset')"
            >
              <template #icon><IconRefresh /></template>
            </a-button>
          </a-tooltip>
        </a-popconfirm>
      </div>
    </header>

    <nav class="instance-tabs" :aria-label="t('workspace.instanceViews')">
      <RouterLink
        v-for="tab in [
          { view: 'overview', icon: IconDashboard, label: t('console.overviewTab') },
          { view: 'console', icon: IconCode, label: t('console.consoleTab') },
          { view: 'config', icon: IconSettings, label: t('console.configTab') },
          { view: 'files', icon: IconFile, label: t('console.filesTab') },
        ]"
        :key="tab.view"
        :class="['instance-tab', { active: activeView === tab.view }]"
        :to="{
          name: 'instance-workspace',
          params: { coreId: core.id, instanceId: instance.id, view: tab.view },
        }"
      >
        <component :is="tab.icon" />
        {{ tab.label }}
      </RouterLink>
    </nav>

    <section class="instance-content">
      <div v-if="activeView === 'overview'" class="overview-view">
        <section class="runtime-grid">
          <article>
            <span>{{ t('console.state') }}</span>
            <i :class="statusClass(instance.runtime.state)"><span></span>{{ statusLabel(instance.runtime.state) }}</i>
          </article>
          <article><span>{{ t('console.pid') }}</span><strong>{{ instance.runtime.pid ?? t('common.none') }}</strong></article>
          <article><span>{{ t('console.startedAt') }}</span><strong>{{ formatDate(instance.runtime.startedAt, locale, t('common.notRecorded')) }}</strong></article>
          <article><span>{{ t('console.exitCode') }}</span><strong>{{ instance.runtime.exitCode ?? t('common.none') }}</strong></article>
          <article><span>{{ t('instances.players') }}</span><strong>{{ instance.runtime.players?.online ?? 0 }}/{{ instance.runtime.players?.max ?? t('common.unknown') }}</strong></article>
          <article><span>{{ t('instances.revision') }}</span><strong>{{ instance.revision }}</strong></article>
        </section>

        <section class="overview-panels">
          <article class="detail-panel">
            <header><h2>{{ t('console.instanceInformation') }}</h2></header>
            <dl>
              <div><dt>{{ t('instances.instanceId') }}</dt><dd>{{ instance.id }}</dd></div>
              <div><dt>{{ t('instances.kind') }}</dt><dd>{{ instance.kind }}</dd></div>
              <div><dt>{{ t('instances.directory') }}</dt><dd>{{ instance.directory }}</dd></div>
              <div><dt>{{ t('console.runtimeMode') }}</dt><dd>{{ instance.launch.runtimeMode ?? 'HOST' }}</dd></div>
              <div><dt>{{ t('console.supervisorMode') }}</dt><dd>{{ instance.launch.supervisorMode ?? 'DIRECT' }}</dd></div>
              <div><dt>{{ t('console.stopTimeout') }}</dt><dd>{{ instance.launch.stopTimeoutSeconds }} s</dd></div>
            </dl>
          </article>
          <article class="detail-panel">
            <header><h2>{{ t('console.launchInformation') }}</h2></header>
            <dl>
              <div><dt>{{ t('console.executable') }}</dt><dd>{{ instance.launch.executable }}</dd></div>
              <div><dt>{{ t('console.arguments') }}</dt><dd>{{ instance.launch.args.join(' ') || t('common.none') }}</dd></div>
              <div><dt>{{ t('console.stopCommand') }}</dt><dd>{{ instance.launch.stopCommand }}</dd></div>
              <div><dt>{{ t('console.environment') }}</dt><dd>{{ Object.keys(instance.launch.environment).length }}</dd></div>
            </dl>
          </article>
        </section>
      </div>

      <div v-else-if="activeView === 'console'" class="console-view">
        <p v-if="errorMessage" class="form-error workspace-message" role="alert">{{ errorMessage }}</p>
        <p v-else-if="noticeMessage" class="notice workspace-message" role="status">{{ noticeMessage }}</p>
        <section class="terminal-shell">
          <header class="terminal-toolbar">
            <span><IconCode /> {{ t('console.output') }} <small>{{ logs.length }}</small></span>
            <a-tooltip :content="t('console.refreshOutput')">
              <a-button type="text" size="mini" :loading="logsLoading" :aria-label="t('console.refreshOutput')" @click="loadLogs(true)">
                <template #icon><IconRefresh /></template>
              </a-button>
            </a-tooltip>
          </header>
          <a-spin class="log-spinner" :loading="logsLoading && !logs.length">
            <div class="log-pane">
              <a-empty v-if="logs.length === 0" :description="t('console.emptyOutput')" />
              <ol v-else>
                <li v-for="line in logs" :key="line.cursor" :class="`stream-${line.stream}`">
                  <time>{{ formatDate(line.occurredAt, locale, t('common.notRecorded'), false) }}</time>
                  <span>{{ line.line }}</span>
                </li>
              </ol>
            </div>
          </a-spin>
        </section>
        <form class="command-row" @submit.prevent="sendCommand">
          <a-input v-model="command" :disabled="commandPending" :placeholder="t('console.commandPlaceholder')">
            <template #prefix><IconCommand /></template>
          </a-input>
          <a-button type="primary" html-type="submit" :loading="commandPending" :disabled="!command.trim()">
            <template #icon><IconSend /></template>
            {{ t('common.send') }}
          </a-button>
        </form>
      </div>

      <ConfigEditor
        v-else-if="activeView === 'config'"
        :client="client"
        :core-id="core.id"
        :instance-id="instance.id"
      />

      <InstanceFileManager
        v-else
        :client="client"
        :core-id="core.id"
        :instance-id="instance.id"
      />
    </section>
  </main>
</template>

<style scoped>
.instance-workspace-page {
  display: flex;
  width: min(100%, 108rem);
  min-height: calc(100vh - 3.5rem);
  flex-direction: column;
  margin: 0 auto;
  padding: 0.85rem 1.5rem 1.25rem;
}

.workspace-breadcrumbs {
  display: flex;
  min-height: 1.6rem;
  align-items: center;
  gap: 0.35rem;
  color: var(--mcnp-text-faint);
  font-size: 0.68rem;
}

.workspace-breadcrumbs a {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  color: var(--mcnp-text-muted);
  text-decoration: none;
}

.workspace-breadcrumbs strong {
  overflow: hidden;
  color: var(--mcnp-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-breadcrumbs > svg {
  font-size: 0.6rem;
}

.instance-heading {
  display: flex;
  min-height: 4.3rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.instance-heading > div,
.lifecycle-actions {
  display: flex;
  align-items: center;
}

.instance-heading > div:first-child {
  min-width: 0;
  gap: 0.75rem;
}

.instance-heading__mark {
  width: 0.25rem;
  height: 2.65rem;
  border-radius: 2px;
  background: var(--mcnp-text-faint);
}

.instance-heading h1,
.instance-heading p {
  overflow: hidden;
  margin: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-heading h1 {
  color: var(--mcnp-text);
  font-size: 1.05rem;
}

.instance-heading p {
  margin-top: 0.25rem;
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
}

.lifecycle-actions {
  flex: 0 0 auto;
  gap: 0.4rem;
}

.instance-tabs {
  display: flex;
  min-height: 2.75rem;
  align-items: stretch;
  gap: 0.2rem;
  border: 1px solid var(--mcnp-border);
  border-bottom: 0;
  border-radius: var(--mcnp-radius) var(--mcnp-radius) 0 0;
  padding: 0 0.5rem;
  background: var(--mcnp-surface-raised);
}

.instance-tab {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  border-bottom: 2px solid transparent;
  padding: 0 0.7rem;
  color: var(--mcnp-text-muted);
  font-size: 0.72rem;
  font-weight: 600;
  text-decoration: none;
}

.instance-tab:hover,
.instance-tab.active {
  color: var(--mcnp-primary);
}

.instance-tab.active {
  border-bottom-color: var(--mcnp-primary);
}

.instance-content {
  display: flex;
  min-height: 35rem;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface);
}

.overview-view {
  display: grid;
  gap: 0.8rem;
  padding: 0.8rem;
}

.runtime-grid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
  background: var(--mcnp-surface-raised);
}

.runtime-grid article {
  display: grid;
  min-width: 0;
  gap: 0.35rem;
  padding: 0.8rem;
  border-right: 1px solid var(--mcnp-border);
}

.runtime-grid article:last-child {
  border-right: 0;
}

.runtime-grid article > span,
.detail-panel dt {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

.runtime-grid strong {
  overflow: hidden;
  color: var(--mcnp-text);
  font-size: 0.7rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overview-panels {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
}

.detail-panel {
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
}

.detail-panel header {
  padding: 0.75rem 0.9rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface-raised);
}

.detail-panel h2 {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.78rem;
}

.detail-panel dl {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0;
  margin: 0;
}

.detail-panel dl div {
  display: grid;
  min-width: 0;
  gap: 0.25rem;
  padding: 0.75rem 0.9rem;
  border-right: 1px solid var(--mcnp-border-subtle);
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.detail-panel dd {
  overflow-wrap: anywhere;
  margin: 0;
  color: var(--mcnp-text-muted);
  font-size: 0.7rem;
}

.console-view {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
}

.terminal-shell {
  display: flex;
  min-height: 24rem;
  flex: 1;
  flex-direction: column;
  margin: 0.75rem 0.75rem 0;
  overflow: hidden;
  border: 1px solid #2c2e33;
  border-radius: 4px 4px 0 0;
  background: var(--mcnp-console);
}

.terminal-toolbar {
  display: flex;
  min-height: 2.25rem;
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
}

.log-spinner {
  display: block;
  min-height: 0;
  flex: 1;
}

.log-spinner :deep(.arco-spin),
.log-spinner :deep(.arco-spin-children) {
  height: 100%;
}

.log-pane {
  height: 100%;
  overflow: auto;
  padding: 0.75rem;
  color: #c9cdd4;
  font-family: "Cascadia Mono", Consolas, monospace;
}

.log-pane ol {
  display: grid;
  gap: 0.2rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.log-pane li {
  display: grid;
  grid-template-columns: 5.5rem minmax(0, 1fr);
  gap: 0.65rem;
  border-left: 2px solid #484b52;
  padding: 0.2rem 0.45rem;
  font-size: 0.69rem;
  line-height: 1.45;
}

.log-pane time {
  color: #70757f;
}

.log-pane li > span {
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

.command-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.55rem;
  padding: 0.65rem 0.75rem 0.75rem;
}

.workspace-message {
  margin: 0.65rem 0.75rem 0;
}

@media (max-width: 64rem) {
  .runtime-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .runtime-grid article:nth-child(3) {
    border-right: 0;
  }

  .overview-panels {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 44rem) {
  .instance-workspace-page {
    padding: 0.7rem 0.75rem 1rem;
  }

  .instance-heading {
    align-items: stretch;
    flex-direction: column;
    padding: 0.6rem 0;
  }

  .lifecycle-actions {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .instance-tabs {
    overflow-x: auto;
  }

  .instance-tab {
    white-space: nowrap;
  }

  .runtime-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .runtime-grid article:nth-child(3) {
    border-right: 1px solid var(--mcnp-border);
  }

  .runtime-grid article:nth-child(2n) {
    border-right: 0;
  }

  .detail-panel dl {
    grid-template-columns: 1fr;
  }

  .log-pane li,
  .command-row {
    grid-template-columns: 1fr;
  }
}
</style>
