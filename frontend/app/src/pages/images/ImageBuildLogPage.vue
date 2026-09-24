<script setup lang="ts">
/** 镜像构建日志：实时构建输出（image-build-log 主题推送）。 */

import { computed, onBeforeUnmount, ref } from 'vue';
import { useRoute } from 'vue-router';
import { LogTerminal, PageHeader } from '@mcnp/ui';
import { useRealtime } from '@/composables';

const realtime = useRealtime();
const route = useRoute();
const buildId = computed(() => route.params.buildId as string);

const terminal = ref<InstanceType<typeof LogTerminal> | null>(null);

const off = realtime.subscribe({ kind: 'image-build-log', buildId: buildId.value }, (event) => {
  terminal.value?.writeLine(event.line);
});
onBeforeUnmount(off);
</script>

<template>
  <div>
    <PageHeader title="镜像构建日志" :subtitle="`构建任务 ${buildId}`" />
    <div class="mcnp-card build-log">
      <LogTerminal ref="terminal" />
    </div>
  </div>
</template>

<style scoped>
.build-log {
  min-height: 480px;
  display: flex;
}

.build-log :deep(.mcnp-log-terminal) {
  flex: 1;
}
</style>
