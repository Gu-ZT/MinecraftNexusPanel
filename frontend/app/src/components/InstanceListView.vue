<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  InputNumber as AInputNumber,
  Modal as AModal,
  Option as AOption,
  Pagination as APagination,
  Popconfirm as APopconfirm,
  Select as ASelect,
  Spin as ASpin,
  Textarea as ATextarea,
  Tooltip as ATooltip,
} from '@arco-design/web-vue';
import {
  IconApps,
  IconCode,
  IconPlayArrow,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconStop,
} from '@arco-design/web-vue/es/icon';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';

import type {
  Core,
  Instance,
  InstanceCreate,
  InstanceKind,
  InstanceState,
  PanelApiClient,
} from '@mcnp/api-client';

import { canStartInstance, canStopInstance, describeError, statusClass } from '../utils/presentation';

const props = defineProps<{
  cores: Core[];
  instances: Instance[];
  client: PanelApiClient;
  loading: boolean;
  actionPending: string | null;
}>();

const emit = defineEmits<{
  action: [action: 'start' | 'stop' | 'kill' | 'reset', coreId: string, instanceId: string];
  created: [instance: Instance];
}>();

const { t, te } = useI18n();
const route = useRoute();
const router = useRouter();
const query = ref('');
const stateFilter = ref('');
const currentPage = ref(1);
const createVisible = ref(false);
const createPending = ref(false);
const createError = ref('');
const createCoreId = ref('');
const createId = ref('');
const createName = ref('');
const createKind = ref<InstanceKind>('PAPER');
const createDirectory = ref('');
const createExecutable = ref('java');
const createArguments = ref('-jar\nserver.jar\nnogui');
const createStopCommand = ref('stop');
const createStopTimeoutSeconds = ref(30);
const pageSize = 12;
const coreFilter = ref(routeCoreId());
const stateOptions: InstanceState[] = [
  'RUNNING',
  'STARTING',
  'STOPPING',
  'STOPPED',
  'FAILED',
  'UNKNOWN',
  'CREATED',
];
const instanceKindOptions: { label: string; value: InstanceKind }[] = [
  { label: 'Vanilla', value: 'VANILLA' },
  { label: 'NeoForge', value: 'NEO_FORGE' },
  { label: 'Forge', value: 'FORGE' },
  { label: 'Fabric', value: 'FABRIC' },
  { label: 'Bukkit', value: 'BUKKIT' },
  { label: 'Spigot', value: 'SPIGOT' },
  { label: 'Paper', value: 'PAPER' },
  { label: 'Purpur', value: 'PURPUR' },
  { label: 'Pufferfish', value: 'PUFFERFISH' },
  { label: 'Folia', value: 'FOLIA' },
  { label: 'Leaf', value: 'LEAF' },
  { label: 'Mohist', value: 'MOHIST' },
  { label: 'Magma', value: 'MAGMA' },
  { label: 'Sponge', value: 'SPONGE' },
  { label: 'Arclight', value: 'ARCLIGHT' },
  { label: 'Youer', value: 'YOUER' },
  { label: 'Silkard', value: 'SILKARD' },
  { label: 'CatServer', value: 'CAT_SERVER' },
  { label: 'Velocity', value: 'VELOCITY' },
  { label: 'Waterfall', value: 'WATERFALL' },
  { label: 'BungeeCord', value: 'BUNGEE_CORD' },
  { label: 'Lightfall', value: 'LIGHTFALL' },
  { label: 'Geyser', value: 'GEYSER' },
  { label: 'Bedrock Dedicated Server', value: 'BEDROCK_DEDICATED_SERVER' },
  { label: 'PocketMine-MP', value: 'POCKET_MINE_MP' },
  { label: 'Nukkit', value: 'NUKKIT' },
  { label: 'Cloudburst Nukkit', value: 'CLOUDBURST_NUKKIT' },
  { label: 'Custom', value: 'CUSTOM' },
];

const filteredInstances = computed(() => {
  const normalizedQuery = query.value.trim().toLocaleLowerCase();
  return props.instances.filter((instance) => {
    if (coreFilter.value && instance.coreId !== coreFilter.value) {
      return false;
    }
    if (stateFilter.value && instance.runtime.state !== stateFilter.value) {
      return false;
    }
    if (!normalizedQuery) {
      return true;
    }
    return [instance.name, instance.id, instance.kind, coreName(instance.coreId)]
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalizedQuery);
  });
});
const pagedInstances = computed(() => {
  const offset = (currentPage.value - 1) * pageSize;
  return filteredInstances.value.slice(offset, offset + pageSize);
});
const canCreateInstance = computed(
  () =>
    /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(createId.value) &&
    createName.value.trim().length > 0 &&
    createDirectory.value.trim().length > 0 &&
    createExecutable.value.trim().length > 0 &&
    createStopCommand.value.trim().length > 0 &&
    createStopTimeoutSeconds.value >= 1 &&
    createStopTimeoutSeconds.value <= 300 &&
    createCoreId.value.length > 0,
);

watch(
  () => route.params.coreId,
  () => {
    coreFilter.value = routeCoreId();
  },
);

watch([query, stateFilter, coreFilter], () => {
  currentPage.value = 1;
});

watch(createId, (value, previousValue) => {
  if (!createDirectory.value || createDirectory.value === `instances/${previousValue}`) {
    createDirectory.value = value ? `instances/${value}` : '';
  }
});

function openCreate(): void {
  createCoreId.value = routeCoreId() || props.cores[0]?.id || '';
  createId.value = '';
  createName.value = '';
  createKind.value = 'PAPER';
  createDirectory.value = '';
  createExecutable.value = 'java';
  createArguments.value = '-jar\nserver.jar\nnogui';
  createStopCommand.value = 'stop';
  createStopTimeoutSeconds.value = 30;
  createError.value = '';
  createVisible.value = true;
}

async function createInstance(): Promise<void> {
  if (!canCreateInstance.value) {
    return;
  }
  createPending.value = true;
  createError.value = '';
  const request: InstanceCreate = {
    id: createId.value,
    name: createName.value.trim(),
    kind: createKind.value,
    directory: createDirectory.value.trim(),
    launch: {
      executable: createExecutable.value.trim(),
      args: createArguments.value
        .split(/\r?\n/u)
        .map((argument) => argument.trim())
        .filter(Boolean),
      environment: {},
      stopCommand: createStopCommand.value.trim(),
      stopTimeoutSeconds: createStopTimeoutSeconds.value,
      runtimeMode: 'HOST',
      supervisorMode: 'DIRECT',
      mcdr: null,
    },
  };
  try {
    const instance = await props.client.createInstance(createCoreId.value, request);
    createVisible.value = false;
    emit('created', instance);
  } catch (error) {
    createError.value = describeError(error, t('error.instanceCreate'));
  } finally {
    createPending.value = false;
  }
}

async function changeCoreFilter(value: unknown): Promise<void> {
  const coreId = typeof value === 'string' ? value : '';
  coreFilter.value = coreId;
  if (coreId) {
    await router.push({ name: 'core-instances', params: { coreId } });
  } else {
    await router.push({ name: 'instances' });
  }
}

function routeCoreId(): string {
  return typeof route.params.coreId === 'string' ? route.params.coreId : '';
}

function coreName(coreId: string): string {
  return props.cores.find((core) => core.id === coreId)?.name ?? coreId;
}

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}

function actionKey(action: string, instance: Instance): string {
  return `${action}:${instance.coreId}:${instance.id}`;
}

function canReset(state: InstanceState): boolean {
  return state === 'FAILED' || state === 'UNKNOWN';
}
</script>

<template>
  <main class="console-page">
    <header class="page-heading page-heading--toolbar">
      <div>
        <p class="page-eyebrow"><IconApps /> {{ t('instances.eyebrow') }}</p>
        <h1>{{ t('instances.title') }}</h1>
      </div>
      <div class="instance-page-actions">
        <p>{{ t('instances.summary', { filtered: filteredInstances.length, total: instances.length }) }}</p>
        <a-button type="primary" :disabled="cores.length === 0" @click="openCreate">
          <template #icon><IconPlus /></template>
          {{ t('instances.create') }}
        </a-button>
      </div>
    </header>

    <section class="filter-bar">
      <a-select
        :model-value="coreFilter"
        :placeholder="t('instances.allCores')"
        allow-clear
        @change="changeCoreFilter"
      >
        <a-option v-for="core in cores" :key="core.id" :value="core.id">{{ core.name }}</a-option>
      </a-select>
      <a-select v-model="stateFilter" :placeholder="t('instances.allStates')" allow-clear>
        <a-option v-for="state in stateOptions" :key="state" :value="state">{{ statusLabel(state) }}</a-option>
      </a-select>
      <a-input v-model="query" allow-clear :placeholder="t('instances.filterPlaceholder')">
        <template #prefix><IconSearch /></template>
      </a-input>
    </section>

    <a-spin class="page-spinner" :loading="loading">
      <section v-if="pagedInstances.length" class="instance-card-grid">
        <article v-for="instance in pagedInstances" :key="`${instance.coreId}:${instance.id}`" class="instance-card">
          <header>
            <span :class="['instance-card__mark', statusClass(instance.runtime.state)]"></span>
            <div>
              <h2>{{ instance.name }}</h2>
              <p>{{ coreName(instance.coreId) }} · {{ instance.kind }}</p>
            </div>
            <i :class="statusClass(instance.runtime.state)"><span></span>{{ statusLabel(instance.runtime.state) }}</i>
          </header>

          <dl>
            <div>
              <dt>{{ t('instances.instanceId') }}</dt>
              <dd>{{ instance.id }}</dd>
            </div>
            <div>
              <dt>{{ t('instances.directory') }}</dt>
              <dd>{{ instance.directory }}</dd>
            </div>
            <div>
              <dt>{{ t('console.pid') }}</dt>
              <dd>{{ instance.runtime.pid ?? t('common.none') }}</dd>
            </div>
            <div>
              <dt>{{ t('instances.players') }}</dt>
              <dd>
                {{ instance.runtime.players?.online ?? 0 }}/{{ instance.runtime.players?.max ?? t('common.unknown') }}
              </dd>
            </div>
          </dl>

          <footer>
            <RouterLink
              class="instance-open-link"
              :to="{
                name: 'instance-workspace',
                params: { coreId: instance.coreId, instanceId: instance.id, view: 'overview' },
              }"
            >
              <a-button type="primary" size="small">
                <template #icon><IconCode /></template>
                {{ t('instances.manage') }}
              </a-button>
            </RouterLink>
            <a-tooltip :content="t('console.start')">
              <a-button
                size="small"
                :loading="actionPending === actionKey('start', instance)"
                :disabled="actionPending !== null || !canStartInstance(instance.runtime.state)"
                :aria-label="t('console.start')"
                @click="emit('action', 'start', instance.coreId, instance.id)"
              >
                <template #icon><IconPlayArrow /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip :content="t('console.stop')">
              <a-button
                size="small"
                :loading="actionPending === actionKey('stop', instance)"
                :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
                :aria-label="t('console.stop')"
                @click="emit('action', 'stop', instance.coreId, instance.id)"
              >
                <template #icon><IconStop /></template>
              </a-button>
            </a-tooltip>
            <a-popconfirm
              :content="t('instances.killConfirm', { name: instance.name })"
              @ok="emit('action', 'kill', instance.coreId, instance.id)"
            >
              <a-tooltip :content="t('console.kill')">
                <a-button
                  size="small"
                  status="danger"
                  :loading="actionPending === actionKey('kill', instance)"
                  :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
                  :aria-label="t('console.kill')"
                >
                  <template #icon><IconStop /></template>
                </a-button>
              </a-tooltip>
            </a-popconfirm>
            <a-popconfirm
              :content="t('instances.resetConfirm', { name: instance.name })"
              @ok="emit('action', 'reset', instance.coreId, instance.id)"
            >
              <a-tooltip :content="t('instances.reset')">
                <a-button
                  size="small"
                  :loading="actionPending === actionKey('reset', instance)"
                  :disabled="actionPending !== null || !canReset(instance.runtime.state)"
                  :aria-label="t('instances.reset')"
                >
                  <template #icon><IconRefresh /></template>
                </a-button>
              </a-tooltip>
            </a-popconfirm>
          </footer>
        </article>
      </section>
      <a-empty v-else :description="instances.length ? t('instances.noMatches') : t('instances.empty')" />
    </a-spin>

    <footer v-if="filteredInstances.length > pageSize" class="page-pagination">
      <a-pagination v-model:current="currentPage" :page-size="pageSize" :total="filteredInstances.length" />
    </footer>

    <a-modal
      v-model:visible="createVisible"
      :title="t('instances.createTitle')"
      :footer="false"
      :width="640"
      unmount-on-close
    >
      <form class="instance-create-form" @submit.prevent="createInstance">
        <div class="instance-create-form__grid">
          <label>
            <span>{{ t('instances.core') }}</span>
            <a-select v-model="createCoreId">
              <a-option v-for="core in cores" :key="core.id" :value="core.id">
                {{ core.name }} · {{ core.status }}
              </a-option>
            </a-select>
          </label>
          <label>
            <span>{{ t('instances.kind') }}</span>
            <a-select v-model="createKind" allow-search>
              <a-option v-for="kind in instanceKindOptions" :key="kind.value" :value="kind.value">
                {{ kind.label }}
              </a-option>
            </a-select>
          </label>
          <label>
            <span>{{ t('instances.instanceId') }}</span>
            <a-input v-model="createId" :max-length="64" allow-clear />
          </label>
          <label>
            <span>{{ t('instances.name') }}</span>
            <a-input v-model="createName" :max-length="128" allow-clear />
          </label>
        </div>
        <label>
          <span>{{ t('instances.directory') }}</span>
          <a-input v-model="createDirectory" :max-length="1024" allow-clear />
        </label>
        <label>
          <span>{{ t('instances.executable') }}</span>
          <a-input v-model="createExecutable" :max-length="4096" allow-clear />
        </label>
        <label>
          <span>{{ t('instances.arguments') }}</span>
          <a-textarea v-model="createArguments" :auto-size="{ minRows: 3, maxRows: 7 }" />
          <small>{{ t('instances.argumentsHint') }}</small>
        </label>
        <div class="instance-create-form__grid">
          <label>
            <span>{{ t('instances.stopCommand') }}</span>
            <a-input v-model="createStopCommand" :max-length="8192" allow-clear />
          </label>
          <label>
            <span>{{ t('instances.stopTimeout') }}</span>
            <a-input-number v-model="createStopTimeoutSeconds" :min="1" :max="300" />
          </label>
        </div>
        <p v-if="createError" class="form-error" role="alert">{{ createError }}</p>
        <div class="instance-create-form__actions">
          <a-button @click="createVisible = false">{{ t('common.cancel') }}</a-button>
          <a-button
            type="primary"
            html-type="submit"
            :loading="createPending"
            :disabled="!canCreateInstance"
          >
            {{ t('instances.create') }}
          </a-button>
        </div>
      </form>
    </a-modal>
  </main>
</template>

<style scoped>
.instance-page-actions,
.instance-create-form__actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.instance-page-actions p {
  margin: 0;
}

.instance-create-form {
  display: grid;
  gap: 1rem;
}

.instance-create-form__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}

.instance-create-form label {
  display: grid;
  min-width: 0;
  gap: 0.4rem;
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.instance-create-form small {
  color: var(--mcnp-text-faint);
  font-weight: 400;
}

.instance-create-form__actions {
  justify-content: flex-end;
  padding-top: 0.25rem;
}

.page-spinner {
  display: block;
  min-height: 18rem;
}

.filter-bar {
  display: grid;
  grid-template-columns: minmax(10rem, 14rem) minmax(9rem, 12rem) minmax(15rem, 1fr);
  gap: 0.65rem;
  margin-bottom: 0.85rem;
}

.filter-bar :deep(.arco-select-view),
.filter-bar :deep(.arco-input-wrapper) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface);
}

.instance-card-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.8rem;
}

.instance-card {
  display: flex;
  min-width: 0;
  min-height: 14.5rem;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.instance-card > header {
  display: grid;
  grid-template-columns: 0.25rem minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.7rem;
  padding: 0.9rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.instance-card__mark {
  width: 0.22rem;
  height: 2.25rem;
  border-radius: 2px;
  background: var(--mcnp-text-faint);
}

.instance-card > header div {
  display: grid;
  min-width: 0;
  gap: 0.2rem;
}

.instance-card h2,
.instance-card p {
  overflow: hidden;
  margin: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-card h2 {
  color: var(--mcnp-text);
  font-size: 0.85rem;
}

.instance-card p {
  color: var(--mcnp-text-faint);
  font-size: 0.65rem;
}

.instance-card dl {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
  margin: 0;
  padding: 0.9rem;
}

.instance-card dl div {
  display: grid;
  min-width: 0;
  gap: 0.22rem;
}

.instance-card dt {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

.instance-card dd {
  overflow: hidden;
  margin: 0;
  color: var(--mcnp-text-muted);
  font-size: 0.7rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-card footer {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-top: auto;
  padding: 0.7rem 0.9rem;
  border-top: 1px solid var(--mcnp-border-subtle);
  background: var(--mcnp-surface-raised);
}

.instance-open-link {
  margin-right: auto;
  text-decoration: none;
}

.page-pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 1rem;
}

@media (max-width: 70rem) {
  .instance-card-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 46rem) {
  .instance-page-actions {
    width: 100%;
    justify-content: space-between;
  }

  .instance-create-form__grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .filter-bar,
  .instance-card-grid {
    grid-template-columns: 1fr;
  }
}
</style>
