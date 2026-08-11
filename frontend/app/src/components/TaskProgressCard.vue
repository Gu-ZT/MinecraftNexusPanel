<script setup lang="ts">
/*
 * TaskProgressCard：右下角浮动任务进度卡（docs/design/frontend-design.md §4.4、§5）。
 * 可收起；任务列表内容由外部通过默认插槽提供，组件不持有任务数据。
 */
import { IconDown, IconUp } from '@arco-design/web-vue/es/icon';
import { ref, watch } from 'vue';

const props = withDefaults(
  defineProps<{
    /** 卡片标题 */
    title: string;
    /** 是否可见；从可见切到不可见时自动展开恢复默认态 */
    visible: boolean;
  }>(),
  { title: '', visible: false },
);

const expanded = ref(true);

watch(
  () => props.visible,
  (visible) => {
    if (!visible) {
      expanded.value = true;
    }
  },
);
</script>

<template>
  <aside v-if="visible" class="task-progress-card" :aria-label="title">
    <header class="task-progress-card__header">
      <strong class="task-progress-card__title">{{ title }}</strong>
      <a-button
        class="task-progress-card__toggle"
        type="text"
        size="mini"
        :aria-expanded="expanded"
        :aria-label="expanded ? $t('common.collapse') : $t('common.expand')"
        @click="expanded = !expanded"
      >
        <template #icon>
          <IconDown v-if="expanded" />
          <IconUp v-else />
        </template>
      </a-button>
    </header>
    <div v-if="expanded" class="task-progress-card__body">
      <slot />
    </div>
  </aside>
</template>

<style scoped>
.task-progress-card {
  position: fixed;
  z-index: 60;
  right: 1.25rem;
  bottom: 1.25rem;
  width: min(22rem, calc(100% - 2.5rem));
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-glass);
  box-shadow: var(--mcnp-shadow-hover);
  backdrop-filter: blur(16px) saturate(1.2);
  -webkit-backdrop-filter: blur(16px) saturate(1.2);
}

.task-progress-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.6rem 0.8rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.task-progress-card__title {
  overflow: hidden;
  color: var(--mcnp-text);
  font-size: 0.78rem;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-progress-card__toggle {
  flex: 0 0 auto;
}

.task-progress-card__body {
  display: grid;
  gap: 0.5rem;
  max-height: 16rem;
  overflow: auto;
  padding: 0.7rem 0.8rem;
}
</style>
