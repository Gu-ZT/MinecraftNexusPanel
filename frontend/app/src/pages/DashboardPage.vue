<script setup lang="ts">
/** 总览：节点健康、运行实例、资源指标与最近任务。 */

import { computed, onBeforeUnmount, reactive } from 'vue';
import { useQuery } from '@tanstack/vue-query';
import { useRouter } from 'vue-router';
import { CoreStatusBadge, InstanceStateTag, MetricSparkline, PageHeader, formatPercent } from '@mcnp/ui';
import { useApi, useRealtime } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const realtime = useRealtime();
const router = useRouter();
const auth = useAuthStore();

const { data: cores } = useQuery({
  queryKey: ['cores'],
  queryFn: () => api.cores.list(),
  enabled: computed(() => auth.has('core.read')),
});

const { data: instances } = useQuery({
  queryKey: ['instances', 'all'],
  queryFn: () => api.instances.list(),
  enabled: computed(() => auth.has('instance.read')),
});

const { data: tasks } = useQuery({
  queryKey: ['tasks'],
  queryFn: () => api.tasks.list(),
});

const running = computed(() => (instances.value ?? []).filter((i) => i.state === 'RUNNING'));
const recentTasks = computed(() => (tasks.value ?? []).slice(0, 5));

// 节点实时指标：core-status 事件写入本地序列
const cpuSeries = reactive<Record<string, number[]>>({});
const off = realtime.subscribe({ kind: 'core-status' }, (event) => {
  const series = cpuSeries[event.coreId] ?? [];
  series.push(event.cpuUsage);
  if (series.length > 40) series.shift();
  cpuSeries[event.coreId] = series;
});
onBeforeUnmount(off);

function coreName(coreId: string): string {
  return cores.value?.find((c) => c.id === coreId)?.name ?? coreId;
}
</script>

<template>
  <div>
    <PageHeader title="总览" subtitle="节点健康、运行实例与近期任务" />

    <div class="stat-row">
      <div class="mcnp-card stat-card">
        <div class="stat-value">{{ cores?.length ?? 0 }}</div>
        <div class="stat-label">Core 节点</div>
      </div>
      <div class="mcnp-card stat-card">
        <div class="stat-value">{{ instances?.length ?? 0 }}</div>
        <div class="stat-label">实例总数</div>
      </div>
      <div class="mcnp-card stat-card">
        <div class="stat-value stat-value--green">{{ running.length }}</div>
        <div class="stat-label">运行中</div>
      </div>
      <div class="mcnp-card stat-card">
        <div class="stat-value">{{ (tasks ?? []).filter((t) => t.status === 'RUNNING').length }}</div>
        <div class="stat-label">进行中任务</div>
      </div>
    </div>

    <div class="dash-grid">
      <div class="mcnp-card">
        <h3>节点状态</h3>
        <ATable
          :data="cores ?? []"
          :pagination="false"
          row-key="id"
          @row-click="(row) => router.push(`/cores/${row.id}`)"
        >
          <template #columns>
            <ATableColumn title="名称" data-index="name" />
            <ATableColumn title="地址" data-index="address">
              <template #cell="{ record }"><span class="mono">{{ record.address }}</span></template>
            </ATableColumn>
            <ATableColumn title="状态">
              <template #cell="{ record }"><CoreStatusBadge :status="record.status" /></template>
            </ATableColumn>
            <ATableColumn title="CPU">
              <template #cell="{ record }">
                <div class="metric-cell">
                  <span>{{ formatPercent(record.cpuUsage) }}</span>
                  <MetricSparkline :data="cpuSeries[record.id] ?? []" :height="32" />
                </div>
              </template>
            </ATableColumn>
            <ATableColumn title="实例" :width="90">
              <template #cell="{ record }">{{ record.runningCount }} / {{ record.instanceCount }}</template>
            </ATableColumn>
          </template>
        </ATable>
      </div>

      <div class="mcnp-card">
        <h3>运行中的实例</h3>
        <AEmpty v-if="running.length === 0" description="暂无运行中的实例" />
        <AList v-else :bordered="false">
          <AListItem v-for="inst in running" :key="inst.id" @click="router.push(`/instances/${inst.id}/console`)">
            <AListItemMeta :title="inst.name" :description="`${coreName(inst.coreId)} · ${inst.serverType} ${inst.version}`" />
            <template #extra><InstanceStateTag :state="inst.state" /></template>
          </AListItem>
        </AList>
      </div>

      <div class="mcnp-card">
        <h3>最近任务</h3>
        <AEmpty v-if="recentTasks.length === 0" description="暂无任务" />
        <ATimeline v-else>
          <ATimelineItem v-for="task in recentTasks" :key="task.id">
            <span>{{ task.title }}</span>
            <ATag
              :color="task.status === 'SUCCESS' ? 'green' : task.status === 'FAILED' ? 'red' : 'arcoblue'"
              size="small"
              style="margin-left: 8px"
            >
              {{ task.status }}
            </ATag>
          </ATimelineItem>
        </ATimeline>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stat-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--mcnp-space-4);
  margin-bottom: var(--mcnp-space-4);
}

.stat-value {
  font-size: 28px;
  font-weight: 600;
}

.stat-value--green {
  color: var(--mcnp-state-running);
}

.stat-label {
  color: var(--mcnp-text-secondary);
  font-size: 13px;
}

.dash-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: var(--mcnp-space-4);
}

h3 {
  margin: 0 0 var(--mcnp-space-3);
  font-size: 15px;
}

.metric-cell {
  display: flex;
  align-items: center;
  gap: var(--mcnp-space-2);
  min-width: 140px;
}
</style>
