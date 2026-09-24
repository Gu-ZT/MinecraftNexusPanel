<script setup lang="ts">
/** 任务中心：异步任务列表、实时进度与取消（tasks 主题推送驱动）。 */

import { computed, onBeforeUnmount } from 'vue';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type Task } from '@mcnp/api-client';
import { PageHeader, TaskProgressCard } from '@mcnp/ui';
import { useApi, useRealtime } from '@/composables';

const api = useApi();
const realtime = useRealtime();
const queryClient = useQueryClient();

const { data: tasks } = useQuery({
  queryKey: ['tasks'],
  queryFn: () => api.tasks.list(),
});

// 实时任务事件：写入 Query 缓存而不是另建 store
const off = realtime.subscribe({ kind: 'tasks' }, (event) => {
  queryClient.setQueryData<Task[]>(['tasks'], (old) => {
    const list = old ?? [];
    const idx = list.findIndex((t) => t.id === event.task.id);
    if (idx < 0) return [event.task, ...list];
    const next = [...list];
    next[idx] = event.task;
    return next;
  });
});
onBeforeUnmount(off);

const grouped = computed(() => {
  const list = tasks.value ?? [];
  return {
    active: list.filter((t) => t.status === 'RUNNING' || t.status === 'PENDING'),
    finished: list.filter((t) => t.status !== 'RUNNING' && t.status !== 'PENDING'),
  };
});

async function cancel(taskId: string): Promise<void> {
  try {
    await api.tasks.cancel(taskId);
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '取消失败');
  }
}
</script>

<template>
  <div>
    <PageHeader title="任务中心" subtitle="耗时操作返回 202 与 taskId，进度经实时通道推送" />
    <h3>进行中</h3>
    <div class="task-grid">
      <TaskProgressCard v-for="task in grouped.active" :key="task.id" :task="task" @cancel="cancel" />
      <AEmpty v-if="grouped.active.length === 0" description="暂无进行中的任务" />
    </div>
    <h3>已完成</h3>
    <div class="task-grid">
      <TaskProgressCard v-for="task in grouped.finished" :key="task.id" :task="task" />
      <AEmpty v-if="grouped.finished.length === 0" description="暂无历史任务" />
    </div>
  </div>
</template>

<style scoped>
h3 {
  margin: var(--mcnp-space-4) 0 var(--mcnp-space-3);
  font-size: 15px;
}

.task-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--mcnp-space-3);
}
</style>
