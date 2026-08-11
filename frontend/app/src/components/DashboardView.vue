<script setup lang="ts">
/*
 * DashboardView：仪表盘（docs/design/frontend-design.md §4.2）。
 * 第一行 KPI 统计卡 + 第二行实例状态分布/节点健康 + 最近审计动态；
 * 数据加载中显示同形骨架屏。只展示后端已交付的数据，不虚构任务/会话指标。
 */
import { Button as AButton, Empty as AEmpty } from '@arco-design/web-vue';
import { IconDownload, IconRight } from '@arco-design/web-vue/es/icon';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import type { Core, Instance, PanelAuditEvent } from '@mcnp/api-client';

import { formatDate } from '../utils/presentation';
import PageHeader from './PageHeader.vue';
import SkeletonBlock from './SkeletonBlock.vue';
import StatCard from './StatCard.vue';
import StatusBadge from './StatusBadge.vue';
import SurfaceCard from './SurfaceCard.vue';

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

/** 实例状态分布：运行中/已停止/失败/其余（启动中、停止中、未知等）归入未分类 */
const instanceDistribution = computed(() => {
  const groups: Array<{ state: string; tone: 'success' | 'neutral' | 'danger' | 'warning' }> = [
    { state: 'RUNNING', tone: 'success' },
    { state: 'STOPPED', tone: 'neutral' },
    { state: 'FAILED', tone: 'danger' },
    { state: 'UNKNOWN', tone: 'warning' },
  ];
  const total = props.instances.length || 1;
  return groups.map((group) => {
    let count = 0;
    for (const instance of props.instances) {
      const state = instance.runtime.state;
      if (state === group.state) {
        count += 1;
      } else if (group.state === 'UNKNOWN' && state !== 'RUNNING' && state !== 'STOPPED' && state !== 'FAILED') {
        count += 1;
      }
    }
    return { ...group, count, percent: Math.round((count / total) * 100) };
  });
});

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
    <PageHeader :eyebrow="t('dashboard.eyebrow')" :title="t('dashboard.title')" :hint="t('dashboard.summary')" />

    <section class="kpi-grid" :aria-label="t('dashboard.metrics')">
      <StatCard
        :label="t('dashboard.onlineCores')"
        :value="`${onlineCoreCount} / ${cores.length}`"
        :hint="t('workspace.coreOnlineSummary', { online: onlineCoreCount, total: cores.length })"
        :tone="onlineCoreCount > 0 ? 'success' : 'neutral'"
        :loading="loading"
      />
      <StatCard
        :label="t('dashboard.runningInstances')"
        :value="`${runningInstanceCount} / ${instances.length}`"
        :hint="t('workspace.instanceRunningSummary', { running: runningInstanceCount, total: instances.length })"
        :tone="runningInstanceCount > 0 ? 'success' : 'neutral'"
        :loading="loading"
      />
      <StatCard
        :label="t('dashboard.recentRequests')"
        :value="String(auditEvents.length)"
        :tone="'neutral'"
        :loading="loading"
      />
      <StatCard :label="t('dashboard.localTime')" :value="formatClock()" :loading="loading" />
    </section>

    <section class="dashboard-grid">
      <SurfaceCard class="panel">
        <header class="panel-head">
          <div>
            <h2>{{ t('dashboard.instanceDistribution') }}</h2>
            <p>{{ t('dashboard.instanceDistributionHint') }}</p>
          </div>
        </header>
        <div v-if="loading" class="dist-skeleton">
          <SkeletonBlock v-for="index in 4" :key="index" width="100%" height="1.1rem" />
        </div>
        <div v-else-if="instances.length" class="dist-list">
          <div v-for="group in instanceDistribution" :key="group.state" class="dist-row">
            <div class="dist-row__meta">
              <StatusBadge :status="group.state" :label="statusLabel(group.state)" :tone="group.tone" />
              <span class="dist-row__count mcnp-num">{{ group.count }}</span>
            </div>
            <div class="dist-bar" role="progressbar" :aria-valuenow="group.percent" aria-valuemin="0" aria-valuemax="100">
              <i class="dist-bar__fill" :class="`dist-bar__fill--${group.tone}`" :style="{ width: `${group.percent}%` }" />
            </div>
          </div>
        </div>
        <a-empty v-else :description="t('instances.empty')" />
      </SurfaceCard>

      <SurfaceCard class="panel">
        <header class="panel-head">
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
        <div v-if="loading" class="dist-skeleton">
          <SkeletonBlock v-for="index in 4" :key="index" width="100%" height="1.1rem" />
        </div>
        <div v-else-if="cores.length" class="node-list">
          <RouterLink
            v-for="core in cores.slice(0, 6)"
            :key="core.id"
            class="node-row"
            :to="{ name: 'core-instances', params: { coreId: core.id } }"
          >
            <span class="node-row__status">
              <StatusBadge :status="core.status" :label="statusLabel(core.status)" />
            </span>
            <span class="node-row__name">{{ core.name }}</span>
            <span class="node-row__detail">
              {{ core.latencyMs === null ? t('common.notRecorded') : `${core.latencyMs} ms` }} ·
              {{ core.version ?? t('common.notRecorded') }}
            </span>
          </RouterLink>
        </div>
        <a-empty v-else :description="t('cores.empty')" />
      </SurfaceCard>
    </section>

    <SurfaceCard class="panel audit-card">
      <header class="panel-head">
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
          <span :class="['audit-code', { 'audit-code--failed': event.statusCode >= 400 }]">{{ event.statusCode }}</span>
          <div class="audit-list__body">
            <strong>{{ event.method }} {{ event.path }}</strong>
            <small>
              {{ formatDate(event.occurredAt, locale, t('common.notRecorded')) }} ·
              {{ event.sourceIp ?? t('common.unknown') }}
            </small>
          </div>
        </li>
      </ol>
      <a-empty v-else :description="t('dashboard.auditEmpty')" />
    </SurfaceCard>
  </main>
</template>

<style scoped>
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.85rem;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(19rem, 0.65fr);
  gap: 0.85rem;
  margin-top: 0.85rem;
}

.audit-card {
  margin-top: 0.85rem;
}

.panel {
  overflow: hidden;
}

.panel-head {
  display: flex;
  min-height: 4rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  border-bottom: 1px solid var(--mcnp-border);
  padding: 0.75rem 1rem;
}

.panel-head div {
  display: grid;
  gap: 0.2rem;
}

.panel-head h2 {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.88rem;
}

.panel-head p {
  margin: 0;
  color: var(--mcnp-text-faint);
  font-size: 0.68rem;
}

.panel-head a {
  text-decoration: none;
}

/* 状态分布 */

.dist-skeleton {
  display: grid;
  gap: 0.8rem;
  padding: 1rem;
}

.dist-list {
  display: grid;
  gap: 0.85rem;
  padding: 1rem 1.1rem;
}

.dist-row {
  display: grid;
  gap: 0.35rem;
}

.dist-row__meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.dist-row__count {
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.dist-bar {
  height: 0.45rem;
  overflow: hidden;
  border-radius: 999px;
  background: var(--mcnp-surface-raised);
}

.dist-bar__fill {
  display: block;
  height: 100%;
  border-radius: 999px;
  transition: width 300ms ease-out;
}

.dist-bar__fill--success {
  background: var(--mcnp-success);
}

.dist-bar__fill--danger {
  background: var(--mcnp-danger);
}

.dist-bar__fill--warning {
  background: var(--mcnp-warning);
}

.dist-bar__fill--neutral {
  background: var(--mcnp-text-faint);
}

/* 节点健康 */

.node-list {
  display: grid;
}

.node-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.7rem;
  padding: 0.68rem 1rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  color: var(--mcnp-text-muted);
  text-decoration: none;
  transition: background-color 150ms ease-out;
}

.node-row:last-child {
  border-bottom: 0;
}

.node-row:hover {
  background: var(--mcnp-surface-hover);
}

.node-row__status {
  display: inline-flex;
}

.node-row__name {
  overflow: hidden;
  color: var(--mcnp-text);
  font-size: 0.76rem;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-row__detail {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
  white-space: nowrap;
}

/* 审计动态 */

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
  border-radius: 6px;
  place-items: center;
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
  font-size: 0.64rem;
  font-weight: 700;
}

.audit-code--failed {
  background: var(--mcnp-danger-soft);
  color: var(--mcnp-danger);
}

.audit-list__body {
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
  .kpi-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .dashboard-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 35rem) {
  .kpi-grid {
    grid-template-columns: 1fr;
  }
}
</style>
