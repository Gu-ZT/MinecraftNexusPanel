<script setup lang="ts">
/** Core 节点健康状态徽标。 */

import { computed } from 'vue';
import { Badge } from '@arco-design/web-vue';
import type { CoreStatus } from '@mcnp/api-client';

const props = defineProps<{ status: CoreStatus }>();

const meta = computed(() => {
  const map: Record<CoreStatus, { label: string; status: 'success' | 'warning' | 'normal' }> = {
    ONLINE: { label: '在线', status: 'success' },
    DEGRADED: { label: '降级', status: 'warning' },
    OFFLINE: { label: '离线', status: 'normal' },
  };
  return map[props.status];
});
</script>

<template>
  <Badge :status="meta.status" :text="meta.label" />
</template>
