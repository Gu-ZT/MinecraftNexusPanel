<script setup lang="ts">
/** 审计日志：操作者、目标、结果、请求 ID 与来源 IP；敏感内容已脱敏。 */

import { computed, ref } from 'vue';
import { useQuery } from '@tanstack/vue-query';
import type { AuditResult } from '@mcnp/api-client';
import { PageHeader, formatTime } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();

const resultFilter = ref<AuditResult | ''>('');
const keyword = ref('');

const { data: events, isLoading } = useQuery({
  queryKey: computed(() => ['audit', resultFilter.value, keyword.value]),
  queryFn: () => api.audit.list({ result: resultFilter.value || undefined, keyword: keyword.value || undefined }),
});

const RESULT_META: Record<AuditResult, { label: string; color: string }> = {
  SUCCESS: { label: '成功', color: 'green' },
  DENIED: { label: '已拒绝', color: 'orange' },
  FAILED: { label: '失败', color: 'red' },
};
</script>

<template>
  <div>
    <PageHeader title="审计日志" subtitle="账户、节点与实例的关键操作记录" />

    <div class="mcnp-card">
      <div class="toolbar">
        <AInputSearch v-model="keyword" placeholder="搜索动作或目标" style="width: 260px" allow-clear />
        <ASelect v-model="resultFilter" style="width: 140px">
          <AOption value="">全部结果</AOption>
          <AOption value="SUCCESS">成功</AOption>
          <AOption value="DENIED">已拒绝</AOption>
          <AOption value="FAILED">失败</AOption>
        </ASelect>
      </div>

      <ATable :data="events ?? []" :loading="isLoading" :pagination="{ pageSize: 20 }" row-key="id" :scroll="{ x: 1040 }">
        <template #columns>
          <ATableColumn title="时间" :width="170">
            <template #cell="{ record }"><span class="mono">{{ formatTime(record.createdAt) }}</span></template>
          </ATableColumn>
          <ATableColumn title="操作者" data-index="actorName" :width="110" />
          <ATableColumn title="动作" :width="170">
            <template #cell="{ record }"><span class="mono">{{ record.action }}</span></template>
          </ATableColumn>
          <ATableColumn title="目标" data-index="target" cell-class="mcnp-td-wrap" />
          <ATableColumn title="结果" :width="90">
            <template #cell="{ record }">
              <ATag :color="RESULT_META[record.result as AuditResult].color" size="small">
                {{ RESULT_META[record.result as AuditResult].label }}
              </ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="请求 ID" :width="110">
            <template #cell="{ record }"><span class="mono">{{ record.requestId }}</span></template>
          </ATableColumn>
          <ATableColumn title="来源 IP" :width="130">
            <template #cell="{ record }"><span class="mono">{{ record.sourceIp }}</span></template>
          </ATableColumn>
        </template>
        <template #expand-row="{ record }">
          <p v-if="record.detail" style="margin: 0">{{ record.detail }}</p>
          <p v-else class="text-secondary" style="margin: 0">无附加信息</p>
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
