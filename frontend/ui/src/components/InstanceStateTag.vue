<script setup lang="ts">
/** 实例状态标签：按 PLAN 第 6 节状态机着色。 */

import { computed } from 'vue';
import { Tag } from '@arco-design/web-vue';
import type { InstanceState } from '@mcnp/api-client';

const props = defineProps<{ state: InstanceState }>();

const meta = computed(() => {
  const map: Record<InstanceState, { label: string; color: string }> = {
    CREATED: { label: '已创建', color: 'gray' },
    STARTING: { label: '启动中', color: 'orange' },
    RUNNING: { label: '运行中', color: 'green' },
    STOPPING: { label: '停止中', color: 'orange' },
    STOPPED: { label: '已停止', color: 'arcoblue' },
    FAILED: { label: '失败', color: 'red' },
  };
  return map[props.state];
});
</script>

<template>
  <Tag :color="meta.color" size="small">{{ meta.label }}</Tag>
</template>
