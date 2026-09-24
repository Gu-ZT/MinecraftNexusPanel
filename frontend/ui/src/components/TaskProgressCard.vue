<script setup lang="ts">
/** 任务进度卡片：任务中心与全局任务提示共用。 */

import { computed } from 'vue';
import { Button, Progress, Tag } from '@arco-design/web-vue';
import type { Task, TaskStatus } from '@mcnp/api-client';
import { formatRelative } from '../format';

const props = defineProps<{ task: Task }>();
const emit = defineEmits<{ cancel: [taskId: string] }>();

const statusMeta = computed(() => {
  const map: Record<TaskStatus, { label: string; color: string }> = {
    PENDING: { label: '排队中', color: 'gray' },
    RUNNING: { label: '进行中', color: 'arcoblue' },
    SUCCESS: { label: '成功', color: 'green' },
    FAILED: { label: '失败', color: 'red' },
    CANCELLED: { label: '已取消', color: 'orange' },
  };
  return map[props.task.status];
});

const progressStatus = computed(() => {
  if (props.task.status === 'FAILED') return 'danger' as const;
  if (props.task.status === 'SUCCESS') return 'success' as const;
  return 'normal' as const;
});
</script>

<template>
  <div class="mcnp-task-card">
    <div class="mcnp-task-card__head">
      <span class="mcnp-task-card__title">{{ task.title }}</span>
      <Tag :color="statusMeta.color" size="small">{{ statusMeta.label }}</Tag>
    </div>
    <Progress
      v-if="task.status === 'RUNNING' || task.status === 'PENDING'"
      :percent="task.progress ?? 0"
      :status="progressStatus"
      size="small"
    />
    <div class="mcnp-task-card__foot">
      <span class="mcnp-task-card__meta">
        {{ formatRelative(task.createdAt) }}<template v-if="task.message"> · {{ task.message }}</template>
      </span>
      <Button
        v-if="task.cancellable && (task.status === 'RUNNING' || task.status === 'PENDING')"
        size="mini"
        status="danger"
        @click="emit('cancel', task.id)"
      >
        取消
      </Button>
    </div>
  </div>
</template>

<style scoped>
.mcnp-task-card {
  padding: var(--mcnp-space-3);
  background: var(--mcnp-bg-elevated);
  border-radius: var(--mcnp-radius-m);
}

.mcnp-task-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--mcnp-space-2);
  margin-bottom: var(--mcnp-space-2);
}

.mcnp-task-card__title {
  color: var(--mcnp-text-primary);
  font-size: 14px;
}

.mcnp-task-card__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: var(--mcnp-space-2);
}

.mcnp-task-card__meta {
  color: var(--mcnp-text-secondary);
  font-size: 12px;
}
</style>
