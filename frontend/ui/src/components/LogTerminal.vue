<script setup lang="ts">
/**
 * 日志终端：xterm.js 封装。
 * 通过 expose 的 writeLine/clear/focus 由父组件喂入实时日志（console 主题事件）。
 */

import { onBeforeUnmount, onMounted, ref } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

const props = withDefaults(defineProps<{ fontSize?: number }>(), { fontSize: 13 });
void props;

const container = ref<HTMLDivElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
let observer: ResizeObserver | null = null;

onMounted(() => {
  if (!container.value) return;
  term = new Terminal({
    fontSize: props.fontSize,
    fontFamily: "var(--mcnp-font-mono)",
    theme: {
      background: '#0d0d10',
      foreground: '#d4d4d8',
      cursor: '#4c9ffe',
      selectionBackground: '#4c9ffe55',
    },
    convertEol: true,
    disableStdin: true,
    scrollback: 5000,
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.open(container.value);
  fit.fit();
  observer = new ResizeObserver(() => fit?.fit());
  observer.observe(container.value);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  term?.dispose();
});

/** 追加一行日志。stream=stderr 时着色为红色。 */
function writeLine(line: string, stream: 'stdout' | 'stderr' = 'stdout'): void {
  if (!term) return;
  if (stream === 'stderr') term.writeln(`\x1b[31m${line}\x1b[0m`);
  else term.writeln(line);
}

function clear(): void {
  term?.clear();
}

function focus(): void {
  term?.focus();
}

defineExpose({ writeLine, clear, focus });
</script>

<template>
  <div ref="container" class="mcnp-log-terminal" />
</template>

<style scoped>
.mcnp-log-terminal {
  width: 100%;
  height: 100%;
  min-height: 320px;
  padding: var(--mcnp-space-2);
  background: var(--mcnp-bg-terminal);
  border-radius: var(--mcnp-radius-s);
}
</style>
