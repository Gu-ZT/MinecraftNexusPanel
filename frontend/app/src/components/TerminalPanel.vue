<script setup lang="ts">
/*
 * TerminalPanel：终端面板容器（docs/design/frontend-design.md §4.4、§5）。
 * 深色面板 + 顶部工具条 + 底部命令输入条；输出与输入内容由外部插槽提供，
 * 组件本身不持有终端状态。
 */
defineProps<{
  /** 工具条左侧标题，可缺省 */
  title?: string;
  /** 面板忙碌（如正在拉取日志）时展示状态点 */
  busy?: boolean;
}>();
</script>

<template>
  <section class="terminal-panel">
    <header v-if="$slots.toolbar || title" class="terminal-panel__toolbar">
      <span v-if="title" class="terminal-panel__title">
        <i v-if="busy" class="terminal-panel__busy" aria-hidden="true" />
        {{ title }}
      </span>
      <div class="terminal-panel__actions">
        <slot name="toolbar" />
      </div>
    </header>
    <div class="terminal-panel__body">
      <slot />
    </div>
    <footer v-if="$slots.input" class="terminal-panel__input">
      <slot name="input" />
    </footer>
  </section>
</template>

<style scoped>
.terminal-panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-console);
}

.terminal-panel__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.45rem 0.75rem;
  color: var(--mcnp-text-muted);
}

.terminal-panel__title {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.72rem;
  font-weight: 600;
}

.terminal-panel__busy {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: var(--mcnp-success);
}

.terminal-panel__actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.terminal-panel__body {
  min-width: 0;
  min-height: 0;
  overflow: auto;
}

.terminal-panel__input {
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}
</style>
