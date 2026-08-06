<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  Option as AOption,
  Pagination as APagination,
  Popconfirm as APopconfirm,
  Select as ASelect,
  Spin as ASpin,
  Tooltip as ATooltip,
} from '@arco-design/web-vue';
import {
  IconApps,
  IconCode,
  IconPlayArrow,
  IconRefresh,
  IconSearch,
  IconStop,
} from '@arco-design/web-vue/es/icon';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';

import type { Core, Instance, InstanceState } from '@mcnp/api-client';

import { canStartInstance, canStopInstance, statusClass } from '../utils/presentation';

const props = defineProps<{
  cores: Core[];
  instances: Instance[];
  loading: boolean;
  actionPending: string | null;
}>();

const emit = defineEmits<{
  action: [action: 'start' | 'stop' | 'kill' | 'reset', coreId: string, instanceId: string];
}>();

const { t, te } = useI18n();
const route = useRoute();
const router = useRouter();
const query = ref('');
const stateFilter = ref('');
const currentPage = ref(1);
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

watch(
  () => route.params.coreId,
  () => {
    coreFilter.value = routeCoreId();
  },
);

watch([query, stateFilter, coreFilter], () => {
  currentPage.value = 1;
});

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
      <p>{{ t('instances.summary', { filtered: filteredInstances.length, total: instances.length }) }}</p>
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
  </main>
</template>

<style scoped>
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
  .filter-bar,
  .instance-card-grid {
    grid-template-columns: 1fr;
  }
}
</style>
