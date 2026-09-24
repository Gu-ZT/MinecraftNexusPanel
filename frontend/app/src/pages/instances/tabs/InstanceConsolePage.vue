<script setup lang="ts">
/** 实例终端：实时 stdout/stderr、stdin 命令、资源指标。 */

import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useQuery } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { LogTerminal, MetricSparkline, formatBytes, formatPercent, type MetricPoint } from '@mcnp/ui';
import { useApi, useRealtime } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const realtime = useRealtime();
const route = useRoute();
const auth = useAuthStore();

const instanceId = computed(() => route.params.id as string);

const { data: instance } = useQuery({
  queryKey: computed(() => ['instances', instanceId.value]),
  queryFn: () => api.instances.get(instanceId.value),
});

const terminal = ref<InstanceType<typeof LogTerminal> | null>(null);
const command = ref('');

// 实时日志流
const offConsole = realtime.subscribe({ kind: 'console', instanceId: instanceId.value }, (event) => {
  terminal.value?.writeLine(event.line, event.stream);
});
onBeforeUnmount(offConsole);

// 实时指标序列（带时间戳，时间窗曲线缺失补 0）
const metrics = reactive({ cpu: [] as MetricPoint[], memoryBytes: 0, playerCount: null as number | null });
const offMetrics = realtime.subscribe({ kind: 'metrics', instanceId: instanceId.value }, (event) => {
  metrics.cpu.push({ at: Date.now(), value: event.cpuUsage });
  if (metrics.cpu.length > 90) metrics.cpu.shift();
  metrics.memoryBytes = event.memoryBytes;
  metrics.playerCount = event.playerCount;
});
onBeforeUnmount(offMetrics);

watch(
  () => instance.value?.state,
  (state) => {
    if (state === 'RUNNING') terminal.value?.writeLine('—— 已连接实例输出流 ——');
  },
);

async function send(): Promise<void> {
  const cmd = command.value.trim();
  if (!cmd) return;
  try {
    await api.instances.sendCommand(instanceId.value, cmd);
    command.value = '';
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '命令发送失败');
  }
}
</script>

<template>
  <div class="console-page">
    <div class="console-metrics">
      <div class="mcnp-card metric">
        <div class="metric__label">CPU</div>
        <!-- metrics 推送周期 2s，桶宽取 3s 覆盖时钟抖动，避免偶发空桶补 0 -->
        <MetricSparkline :points="metrics.cpu" :window-ms="60_000" :bucket-ms="3_000" :height="48" />
      </div>
      <div class="mcnp-card metric">
        <div class="metric__label">内存</div>
        <div class="metric__value">{{ metrics.memoryBytes > 0 ? formatBytes(metrics.memoryBytes) : '—' }}</div>
      </div>
      <div class="mcnp-card metric">
        <div class="metric__label">在线玩家</div>
        <div class="metric__value">{{ metrics.playerCount ?? '—' }}</div>
      </div>
      <div class="mcnp-card metric">
        <div class="metric__label">CPU 占用</div>
        <div class="metric__value">{{ metrics.cpu.length > 0 ? formatPercent(metrics.cpu[metrics.cpu.length - 1]?.value ?? 0) : '—' }}</div>
      </div>
    </div>

    <div class="mcnp-card console-body">
      <LogTerminal ref="terminal" />
      <div v-if="auth.has('instance.console.write')" class="console-input">
        <AInput
          v-model="command"
          class="mono"
          placeholder="输入命令并回车，如 say hello / list / stop"
          :disabled="instance?.state !== 'RUNNING'"
          @press-enter="send"
        >
          <template #prepend>&gt;</template>
        </AInput>
      </div>
    </div>
  </div>
</template>

<style scoped>
.console-metrics {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr;
  gap: var(--mcnp-space-3);
  margin-bottom: var(--mcnp-space-3);
}

.metric__label {
  font-size: 12px;
  color: var(--mcnp-text-secondary);
  margin-bottom: var(--mcnp-space-1);
}

.metric__value {
  font-size: 20px;
  font-weight: 600;
}

.console-body {
  display: flex;
  flex-direction: column;
  gap: var(--mcnp-space-3);
  min-height: 480px;
}

.console-body :deep(.mcnp-log-terminal) {
  flex: 1;
}

.console-input {
  flex-shrink: 0;
}

@media (max-width: 900px) {
  .console-metrics {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
