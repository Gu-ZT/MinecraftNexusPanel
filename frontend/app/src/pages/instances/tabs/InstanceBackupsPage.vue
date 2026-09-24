<script setup lang="ts">
/**
 * 实例备份页（原型）。
 * PLAN M3 将备份/恢复纳入任务中心；本页展示备份类任务并提供触发入口。
 * TODO(M3): 备份列表 API（快照清单、恢复、下载）。
 */

import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useQuery } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { PermissionGate, formatTime } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();
const route = useRoute();
const instanceId = computed(() => route.params.id as string);

// 原型阶段以任务中心的 BACKUP 类任务近似备份历史
const { data: tasks } = useQuery({ queryKey: ['tasks'], queryFn: () => api.tasks.list() });

const backups = computed(() =>
  (tasks.value ?? []).filter((t) => t.kind === 'BACKUP' && t.instanceId === instanceId.value),
);

function triggerBackup(): void {
  Message.info('原型提示：备份将经任务中心异步执行（PLAN M3）');
}
</script>

<template>
  <div class="mcnp-card">
    <div class="toolbar">
      <PermissionGate when="instance.control">
        <AButton type="primary" @click="triggerBackup">立即备份</AButton>
      </PermissionGate>
    </div>
    <ATable :data="backups" :pagination="false" row-key="id">
      <template #columns>
        <ATableColumn title="备份任务" data-index="title" />
        <ATableColumn title="结果" :width="100">
          <template #cell="{ record }">
            <ATag :color="record.status === 'SUCCESS' ? 'green' : 'red'" size="small">{{ record.status }}</ATag>
          </template>
        </ATableColumn>
        <ATableColumn title="产物">
          <template #cell="{ record }"><span class="mono">{{ record.message ?? '—' }}</span></template>
        </ATableColumn>
        <ATableColumn title="时间" :width="170">
          <template #cell="{ record }">{{ formatTime(record.finishedAt ?? record.createdAt) }}</template>
        </ATableColumn>
      </template>
    </ATable>
    <p class="text-secondary" style="margin-top: 12px">
      原型说明：备份快照清单、恢复与下载将随 M3「备份/恢复」能力接入，当前以任务记录近似展示。
    </p>
  </div>
</template>

<style scoped>
.toolbar {
  margin-bottom: var(--mcnp-space-3);
}
</style>
