<script setup lang="ts">
/** Core 节点详情：连接信息、CPU 拓扑与该节点上的实例。 */

import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useQuery } from '@tanstack/vue-query';
import { CoreStatusBadge, InstanceStateTag, PageHeader, formatPercent, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();
const route = useRoute();
const router = useRouter();
const coreId = computed(() => route.params.id as string);

const { data: core } = useQuery({
  queryKey: ['cores', coreId],
  queryFn: () => api.cores.get(coreId.value),
});

const { data: topology } = useQuery({
  queryKey: ['cores', coreId, 'cpu-topology'],
  queryFn: () => api.cores.cpuTopology(coreId.value),
});

const { data: instances } = useQuery({
  queryKey: ['instances', { coreId }],
  queryFn: () => api.instances.list({ coreId: coreId.value }),
});

const coreLabel = computed(() => core.value?.name ?? coreId.value);
</script>

<template>
  <div>
    <PageHeader :title="coreLabel" :subtitle="core ? `${core.address} · ${core.os}/${core.arch} · v${core.version}` : ''">
      <template #extra>
        <CoreStatusBadge v-if="core" :status="core.status" />
      </template>
    </PageHeader>

    <div class="detail-grid">
      <div class="mcnp-card">
        <h3>资源用量</h3>
        <div v-if="core" class="usage">
          <div>CPU <span class="mono">{{ formatPercent(core.cpuUsage) }}</span></div>
          <AProgress :percent="core.cpuUsage" :show-text="false" />
          <div>内存 <span class="mono">{{ formatPercent(core.memoryUsage) }}</span></div>
          <AProgress :percent="core.memoryUsage" :show-text="false" color="#23c343" />
          <p class="text-secondary">最近心跳：{{ formatRelative(core.lastSeenAt) }}</p>
        </div>
      </div>

      <div class="mcnp-card">
        <h3>CPU 拓扑（大核调度）</h3>
        <template v-if="topology">
          <p>逻辑 CPU：{{ topology.logicalCpus }} 个</p>
          <p>
            性能核：
            <ATag v-for="cpu in topology.performanceCores" :key="cpu" color="arcoblue" size="small" class="cpu-tag">{{ cpu }}</ATag>
          </p>
          <p>
            能效核：
            <ATag v-for="cpu in topology.efficiencyCores" :key="cpu" size="small" class="cpu-tag">{{ cpu }}</ATag>
          </p>
          <p v-for="node in topology.numaNodes" :key="node.id" class="text-secondary">
            NUMA 节点 {{ node.id }}：{{ node.cpus.length }} 个逻辑 CPU
          </p>
        </template>
        <AEmpty v-else description="该节点未上报 CPU 拓扑" />
      </div>

      <div class="mcnp-card detail-grid__full">
        <h3>该节点上的实例</h3>
        <ATable :data="instances ?? []" :pagination="false" row-key="id">
          <template #columns>
            <ATableColumn title="名称" data-index="name" />
            <ATableColumn title="类型" :width="140">
              <template #cell="{ record }">{{ record.serverType }} {{ record.version }}</template>
            </ATableColumn>
            <ATableColumn title="状态" :width="100">
              <template #cell="{ record }"><InstanceStateTag :state="record.state" /></template>
            </ATableColumn>
            <ATableColumn title="工作目录">
              <template #cell="{ record }"><span class="mono">{{ record.workDir }}</span></template>
            </ATableColumn>
            <ATableColumn title="操作" :width="90">
              <template #cell="{ record }">
                <AButton size="small" @click="router.push(`/instances/${record.id}/console`)">打开</AButton>
              </template>
            </ATableColumn>
          </template>
        </ATable>
      </div>
    </div>
  </div>
</template>

<style scoped>
.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--mcnp-space-4);
}

.detail-grid__full {
  grid-column: 1 / -1;
}

h3 {
  margin: 0 0 var(--mcnp-space-3);
  font-size: 15px;
}

.usage > div {
  margin: var(--mcnp-space-2) 0 var(--mcnp-space-1);
}

.cpu-tag {
  margin-right: 4px;
}
</style>
