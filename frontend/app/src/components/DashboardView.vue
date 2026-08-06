<script setup lang="ts">
import { Button as AButton, Empty as AEmpty, Spin as ASpin } from '@arco-design/web-vue';
import {
  IconApps,
  IconClockCircle,
  IconCloud,
  IconDashboard,
  IconDownload,
  IconRight,
  IconSafe,
} from '@arco-design/web-vue/es/icon';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import type { Core, Instance, PanelAuditEvent } from '@mcnp/api-client';

import { formatDate, statusClass } from '../utils/presentation';

const props = defineProps<{
  cores: Core[];
  instances: Instance[];
  auditEvents: PanelAuditEvent[];
  loading: boolean;
  canExportAudit: boolean;
  exportingAudit: boolean;
}>();

const emit = defineEmits<{
  exportAudit: [];
}>();

const { locale, t, te } = useI18n();
const now = ref(new Date());
let clockTimer: number | undefined;

const onlineCoreCount = computed(() => props.cores.filter((core) => core.status === 'ONLINE').length);
const runningInstanceCount = computed(
  () => props.instances.filter((instance) => instance.runtime.state === 'RUNNING').length,
);
const recentAuditEvents = computed(() => props.auditEvents.slice(0, 8));

onMounted(() => {
  clockTimer = window.setInterval(() => {
    now.value = new Date();
  }, 1000);
});

onUnmounted(() => {
  if (clockTimer !== undefined) {
    window.clearInterval(clockTimer);
  }
});

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}

function formatClock(): string {
  return new Intl.DateTimeFormat(locale.value, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(now.value);
}
</script>

<template>
  <main class="console-page">
    <header class="page-heading">
      <div>
        <p class="page-eyebrow"><IconDashboard /> {{ t('dashboard.eyebrow') }}</p>
        <h1>{{ t('dashboard.title') }}</h1>
      </div>
      <p>{{ t('dashboard.summary') }}</p>
    </header>

    <a-spin class="page-spinner" :loading="loading">
      <section class="metric-grid" :aria-label="t('dashboard.metrics')">
        <article class="metric-card">
          <span class="metric-card__icon core"><IconCloud /></span>
          <div>
            <span>{{ t('dashboard.onlineCores') }}</span>
            <strong>{{ onlineCoreCount }}<small>/ {{ cores.length }}</small></strong>
          </div>
        </article>
        <article class="metric-card">
          <span class="metric-card__icon instance"><IconApps /></span>
          <div>
            <span>{{ t('dashboard.runningInstances') }}</span>
            <strong>{{ runningInstanceCount }}<small>/ {{ instances.length }}</small></strong>
          </div>
        </article>
        <article class="metric-card">
          <span class="metric-card__icon audit"><IconSafe /></span>
          <div>
            <span>{{ t('dashboard.recentRequests') }}</span>
            <strong>{{ auditEvents.length }}</strong>
          </div>
        </article>
        <article class="metric-card">
          <span class="metric-card__icon clock"><IconClockCircle /></span>
          <div>
            <span>{{ t('dashboard.localTime') }}</span>
            <strong class="metric-clock">{{ formatClock() }}</strong>
          </div>
        </article>
      </section>

      <section class="dashboard-grid">
        <article class="data-panel">
          <header class="data-panel__heading">
            <div>
              <h2>{{ t('dashboard.nodeStatus') }}</h2>
              <p>{{ t('dashboard.nodeStatusHint') }}</p>
            </div>
            <RouterLink :to="{ name: 'nodes' }">
              <a-button type="text" size="small">
                {{ t('common.viewAll') }}
                <template #icon><IconRight /></template>
              </a-button>
            </RouterLink>
          </header>
          <div v-if="cores.length" class="data-table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>{{ t('nodes.name') }}</th>
                  <th>{{ t('common.status') }}</th>
                  <th>{{ t('nodes.latency') }}</th>
                  <th>{{ t('nodes.version') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="core in cores.slice(0, 6)" :key="core.id">
                  <td>
                    <RouterLink :to="{ name: 'core-instances', params: { coreId: core.id } }">
                      {{ core.name }}
                    </RouterLink>
                    <small>{{ core.address }}</small>
                  </td>
                  <td><i :class="statusClass(core.status)"><span></span>{{ statusLabel(core.status) }}</i></td>
                  <td>{{ core.latencyMs === null ? t('common.notRecorded') : `${core.latencyMs} ms` }}</td>
                  <td>{{ core.version ?? t('common.notRecorded') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <a-empty v-else :description="t('cores.empty')" />
        </article>

        <article class="data-panel">
          <header class="data-panel__heading">
            <div>
              <h2>{{ t('dashboard.audit') }}</h2>
              <p>{{ t('dashboard.auditHint') }}</p>
            </div>
            <a-button
              v-if="canExportAudit"
              type="text"
              size="small"
              :loading="exportingAudit"
              @click="emit('exportAudit')"
            >
              {{ t('dashboard.exportAudit') }}
              <template #icon><IconDownload /></template>
            </a-button>
          </header>
          <ol v-if="recentAuditEvents.length" class="audit-list">
            <li v-for="event in recentAuditEvents" :key="event.id">
              <span :class="['audit-code', { failed: event.statusCode >= 400 }]">{{ event.statusCode }}</span>
              <div>
                <strong>{{ event.method }} {{ event.path }}</strong>
                <small>
                  {{ formatDate(event.occurredAt, locale, t('common.notRecorded')) }} ·
                  {{ event.sourceIp ?? t('common.unknown') }}
                </small>
              </div>
            </li>
          </ol>
          <a-empty v-else :description="t('dashboard.auditEmpty')" />
        </article>
      </section>
    </a-spin>
  </main>
</template>

<style scoped>
.page-spinner {
  display: block;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.85rem;
}

.metric-card {
  display: grid;
  min-height: 6.75rem;
  grid-template-columns: 2.7rem minmax(0, 1fr);
  align-items: center;
  gap: 0.85rem;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  padding: 1rem;
  background: var(--mcnp-surface);
}

.metric-card__icon {
  display: grid;
  width: 2.7rem;
  height: 2.7rem;
  border-radius: 6px;
  place-items: center;
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
  font-size: 1.25rem;
}

.metric-card__icon.core {
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
}

.metric-card__icon.audit {
  background: var(--mcnp-warning-soft);
  color: var(--mcnp-warning);
}

.metric-card__icon.clock {
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
}

.metric-card div {
  display: grid;
  min-width: 0;
  gap: 0.35rem;
}

.metric-card span {
  color: var(--mcnp-text-muted);
  font-size: 0.72rem;
}

.metric-card strong {
  color: var(--mcnp-text);
  font-size: 1.65rem;
  font-weight: 650;
}

.metric-card strong small {
  margin-left: 0.25rem;
  color: var(--mcnp-text-faint);
  font-size: 0.75rem;
  font-weight: 500;
}

.metric-card .metric-clock {
  font-size: 1.25rem;
  font-variant-numeric: tabular-nums;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(19rem, 0.65fr);
  gap: 0.85rem;
  margin-top: 0.85rem;
}

.data-panel {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.data-panel__heading {
  display: flex;
  min-height: 4rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--mcnp-border);
}

.data-panel__heading div {
  display: grid;
  gap: 0.2rem;
}

.data-panel__heading h2 {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.88rem;
}

.data-panel__heading p {
  margin: 0;
  color: var(--mcnp-text-faint);
  font-size: 0.68rem;
}

.data-panel__heading a {
  text-decoration: none;
}

.data-table-wrap {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.72rem;
}

.data-table th,
.data-table td {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  color: var(--mcnp-text-muted);
  text-align: left;
}

.data-table th {
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
  font-weight: 600;
}

.data-table tr:last-child td {
  border-bottom: 0;
}

.data-table td:first-child {
  display: grid;
  min-width: 12rem;
  gap: 0.2rem;
}

.data-table a {
  color: var(--mcnp-text);
  font-weight: 600;
  text-decoration: none;
}

.data-table small {
  color: var(--mcnp-text-faint);
}

.audit-list {
  display: grid;
  margin: 0;
  padding: 0;
  list-style: none;
}

.audit-list li {
  display: grid;
  grid-template-columns: 2.5rem minmax(0, 1fr);
  align-items: center;
  gap: 0.7rem;
  padding: 0.68rem 0.9rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.audit-list li:last-child {
  border-bottom: 0;
}

.audit-code {
  display: inline-grid;
  height: 1.35rem;
  border-radius: 4px;
  place-items: center;
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
  font-size: 0.64rem;
  font-weight: 700;
}

.audit-code.failed {
  background: var(--mcnp-danger-soft);
  color: var(--mcnp-danger);
}

.audit-list div {
  display: grid;
  min-width: 0;
  gap: 0.18rem;
}

.audit-list strong,
.audit-list small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.audit-list strong {
  color: var(--mcnp-text);
  font-size: 0.7rem;
  font-weight: 600;
}

.audit-list small {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

@media (max-width: 64rem) {
  .metric-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .dashboard-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 35rem) {
  .metric-grid {
    grid-template-columns: 1fr;
  }
}
</style>
