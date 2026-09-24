<script setup lang="ts">
/** 实例列表：按 Core/状态/标签过滤，跟随全局 Core 切换器。 */

import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useQuery } from '@tanstack/vue-query';
import type { InstanceState } from '@mcnp/api-client';
import { InstanceStateTag, PageHeader, PermissionGate, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useCoreStore } from '@/stores/core';

const api = useApi();
const router = useRouter();
const coreStore = useCoreStore();

const stateFilter = ref<InstanceState | ''>('');
const keyword = ref('');

const { data: cores } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });

const { data: instances, isLoading } = useQuery({
  queryKey: computed(() => ['instances', coreStore.selectedCoreId, stateFilter.value, keyword.value]),
  queryFn: () =>
    api.instances.list({
      coreId: coreStore.selectedCoreId ?? undefined,
      state: stateFilter.value || undefined,
      keyword: keyword.value || undefined,
    }),
});

function coreName(coreId: string): string {
  return cores.value?.find((c) => c.id === coreId)?.name ?? coreId;
}

const STATE_OPTIONS: { value: InstanceState | ''; label: string }[] = [
  { value: '', label: '全部状态' },
  { value: 'RUNNING', label: '运行中' },
  { value: 'STOPPED', label: '已停止' },
  { value: 'STARTING', label: '启动中' },
  { value: 'STOPPING', label: '停止中' },
  { value: 'CREATED', label: '已创建' },
  { value: 'FAILED', label: '失败' },
];
</script>

<template>
  <div>
    <PageHeader title="实例" subtitle="每台实例归属于且仅归属于一个 Core">
      <template #extra>
        <PermissionGate when="instance.create">
          <AButton type="primary" @click="router.push('/instances/new')">一键搭建</AButton>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card">
      <div class="toolbar">
        <AInputSearch v-model="keyword" placeholder="搜索实例名称" style="width: 240px" allow-clear />
        <ASelect v-model="stateFilter" style="width: 140px">
          <AOption v-for="opt in STATE_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</AOption>
        </ASelect>
      </div>

      <ATable :data="instances ?? []" :loading="isLoading" :pagination="false" row-key="id">
        <template #columns>
          <ATableColumn title="名称" data-index="name" />
          <ATableColumn title="节点" :width="150">
            <template #cell="{ record }">{{ coreName(record.coreId) }}</template>
          </ATableColumn>
          <ATableColumn title="类型" :width="130">
            <template #cell="{ record }">{{ record.serverType }} {{ record.version }}</template>
          </ATableColumn>
          <ATableColumn title="状态" :width="100">
            <template #cell="{ record }"><InstanceStateTag :state="record.state" /></template>
          </ATableColumn>
          <ATableColumn title="标签">
            <template #cell="{ record }">
              <ATag v-for="tag in record.tags" :key="tag" size="small" style="margin-right: 4px">{{ tag }}</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="到期时间" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.expiresAt) }}</template>
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
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: var(--mcnp-space-3);
  margin-bottom: var(--mcnp-space-3);
}
</style>
