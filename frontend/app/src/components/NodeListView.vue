<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  Modal as AModal,
  Option as AOption,
  Select as ASelect,
  Spin as ASpin,
} from '@arco-design/web-vue';
import { IconApps, IconCloud, IconSearch, IconStorage } from '@arco-design/web-vue/es/icon';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import type { Core, CpuTopology, PanelApiClient } from '@mcnp/api-client';

import { describeError, formatDate, statusClass } from '../utils/presentation';

const props = defineProps<{
  client: PanelApiClient;
  cores: Core[];
  loading: boolean;
}>();

const { locale, t, te } = useI18n();
const query = ref('');
const statusFilter = ref('');
const topologyVisible = ref(false);
const topologyLoading = ref(false);
const topology = ref<CpuTopology | null>(null);
const topologyCore = ref<Core | null>(null);
const topologyError = ref('');

const filteredCores = computed(() => {
  const normalizedQuery = query.value.trim().toLocaleLowerCase();
  return props.cores.filter((core) => {
    if (statusFilter.value && core.status !== statusFilter.value) {
      return false;
    }
    if (!normalizedQuery) {
      return true;
    }
    return [core.name, core.id, core.address, core.version ?? '', ...core.tags]
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalizedQuery);
  });
});

async function openTopology(core: Core): Promise<void> {
  topologyVisible.value = true;
  topologyLoading.value = true;
  topology.value = null;
  topologyCore.value = core;
  topologyError.value = '';
  try {
    topology.value = await props.client.getCpuTopology(core.id);
  } catch (error) {
    topologyError.value = describeError(error, t('error.cpuTopology'));
  } finally {
    topologyLoading.value = false;
  }
}

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}

function isolationLabel(value: boolean | null): string {
  if (value === null) {
    return t('common.unknown');
  }
  return value ? t('nodes.isolated') : t('nodes.shared');
}
</script>

<template>
  <main class="console-page">
    <header class="page-heading page-heading--toolbar">
      <div>
        <p class="page-eyebrow"><IconCloud /> {{ t('nodes.eyebrow') }}</p>
        <h1>{{ t('nodes.title') }}</h1>
      </div>
      <p>{{ t('nodes.summary', { filtered: filteredCores.length, total: cores.length }) }}</p>
    </header>

    <section class="filter-bar">
      <a-select v-model="statusFilter" :placeholder="t('nodes.allStates')" allow-clear>
        <a-option value="ONLINE">{{ statusLabel('ONLINE') }}</a-option>
        <a-option value="OFFLINE">{{ statusLabel('OFFLINE') }}</a-option>
        <a-option value="DEGRADED">{{ statusLabel('DEGRADED') }}</a-option>
        <a-option value="AUTH_FAILED">{{ statusLabel('AUTH_FAILED') }}</a-option>
        <a-option value="INCOMPATIBLE">{{ statusLabel('INCOMPATIBLE') }}</a-option>
      </a-select>
      <a-input v-model="query" allow-clear :placeholder="t('nodes.searchPlaceholder')">
        <template #prefix><IconSearch /></template>
      </a-input>
    </section>

    <a-spin class="page-spinner" :loading="loading">
      <section v-if="filteredCores.length" class="node-grid">
        <article v-for="core in filteredCores" :key="core.id" class="node-card">
          <header>
            <span class="node-icon"><IconCloud /></span>
            <div>
              <h2>{{ core.name }}</h2>
              <p>{{ core.address }}</p>
            </div>
            <i :class="statusClass(core.status)"><span></span>{{ statusLabel(core.status) }}</i>
          </header>
          <dl>
            <div><dt>{{ t('nodes.latency') }}</dt><dd>{{ core.latencyMs === null ? t('common.notRecorded') : `${core.latencyMs} ms` }}</dd></div>
            <div><dt>{{ t('nodes.version') }}</dt><dd>{{ core.version ?? t('common.notRecorded') }}</dd></div>
            <div><dt>{{ t('nodes.protocol') }}</dt><dd>{{ core.protocolVersion ?? t('common.notRecorded') }}</dd></div>
            <div><dt>{{ t('nodes.lastSeen') }}</dt><dd>{{ formatDate(core.lastSeenAt, locale, t('common.notRecorded')) }}</dd></div>
            <div><dt>{{ t('nodes.certificate') }}</dt><dd>{{ core.certificateVerified === null ? t('common.unknown') : core.certificateVerified ? t('nodes.verified') : t('nodes.unverified') }}</dd></div>
            <div><dt>{{ t('nodes.capabilities') }}</dt><dd>{{ core.capabilities.length }}</dd></div>
          </dl>
          <footer>
            <RouterLink :to="{ name: 'core-instances', params: { coreId: core.id } }">
              <a-button size="small">
                <template #icon><IconApps /></template>
                {{ t('nodes.instances') }}
              </a-button>
            </RouterLink>
            <a-button size="small" :disabled="core.status !== 'ONLINE'" @click="openTopology(core)">
              <template #icon><IconStorage /></template>
              {{ t('nodes.cpuTopology') }}
            </a-button>
          </footer>
        </article>
      </section>
      <a-empty v-else :description="cores.length ? t('nodes.noMatches') : t('cores.empty')" />
    </a-spin>

    <a-modal v-model:visible="topologyVisible" :footer="false" :width="760" unmount-on-close>
      <template #title>{{ t('nodes.topologyTitle', { name: topologyCore?.name ?? '' }) }}</template>
      <a-spin class="topology-spinner" :loading="topologyLoading">
        <p v-if="topologyError" class="form-error" role="alert">{{ topologyError }}</p>
        <div v-else-if="topology" class="topology-content">
          <dl class="topology-summary">
            <div><dt>{{ t('nodes.architecture') }}</dt><dd>{{ topology.architecture }}</dd></div>
            <div><dt>{{ t('nodes.logicalCpus') }}</dt><dd>{{ topology.logicalCpus.length }}</dd></div>
            <div><dt>{{ t('nodes.physicalCores') }}</dt><dd>{{ topology.physicalCoreCount ?? t('common.unknown') }}</dd></div>
            <div><dt>{{ t('nodes.detection') }}</dt><dd>{{ topology.detection.source }} · {{ topology.detection.confidence }}</dd></div>
          </dl>
          <div class="cpu-grid">
            <article v-for="cpu in topology.logicalCpus" :key="cpu.id" :class="['cpu-cell', { offline: !cpu.online }]">
              <strong>CPU {{ cpu.id }}</strong>
              <span>{{ cpu.performanceClass }}</span>
              <small>{{ isolationLabel(cpu.isolated) }} · NUMA {{ cpu.numaNode ?? t('common.unknown') }}</small>
            </article>
          </div>
        </div>
      </a-spin>
    </a-modal>
  </main>
</template>

<style scoped>
.page-spinner,
.topology-spinner {
  display: block;
  min-height: 12rem;
}

.filter-bar {
  display: grid;
  grid-template-columns: minmax(10rem, 13rem) minmax(15rem, 1fr);
  gap: 0.65rem;
  margin-bottom: 0.85rem;
}

.filter-bar :deep(.arco-select-view),
.filter-bar :deep(.arco-input-wrapper) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface);
}

.node-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.85rem;
}

.node-card {
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.node-card header {
  display: grid;
  grid-template-columns: 2.5rem minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.75rem;
  padding: 0.9rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.node-icon {
  display: grid;
  width: 2.5rem;
  height: 2.5rem;
  border-radius: 5px;
  place-items: center;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
  font-size: 1.1rem;
}

.node-card header div {
  display: grid;
  min-width: 0;
  gap: 0.2rem;
}

.node-card h2,
.node-card p {
  overflow: hidden;
  margin: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-card h2 {
  color: var(--mcnp-text);
  font-size: 0.86rem;
}

.node-card p {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
}

.node-card dl,
.topology-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.8rem;
  margin: 0;
  padding: 0.9rem;
}

.node-card dl div,
.topology-summary div {
  display: grid;
  min-width: 0;
  gap: 0.25rem;
}

.node-card dt,
.topology-summary dt {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

.node-card dd,
.topology-summary dd {
  overflow: hidden;
  margin: 0;
  color: var(--mcnp-text-muted);
  font-size: 0.7rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-card footer {
  display: flex;
  gap: 0.5rem;
  padding: 0.7rem 0.9rem;
  border-top: 1px solid var(--mcnp-border-subtle);
  background: var(--mcnp-surface-raised);
}

.node-card footer a {
  text-decoration: none;
}

.topology-content {
  display: grid;
  gap: 0.8rem;
}

.topology-summary {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface-raised);
}

.cpu-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.45rem;
  max-height: 24rem;
  overflow: auto;
}

.cpu-cell {
  display: grid;
  gap: 0.25rem;
  border: 1px solid var(--mcnp-border);
  border-left: 3px solid var(--mcnp-success);
  border-radius: 4px;
  padding: 0.65rem;
  background: var(--mcnp-surface);
}

.cpu-cell.offline {
  border-left-color: var(--mcnp-danger);
  opacity: 0.65;
}

.cpu-cell strong {
  color: var(--mcnp-text);
  font-size: 0.72rem;
}

.cpu-cell span,
.cpu-cell small {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

@media (max-width: 50rem) {
  .filter-bar,
  .node-grid {
    grid-template-columns: 1fr;
  }

  .cpu-grid,
  .topology-summary {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
