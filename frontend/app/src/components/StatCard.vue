<script setup lang="ts">
/*
 * StatCard：KPI 统计卡（docs/design/frontend-design.md §4.2、§5）。
 * 顶部渐变指示条 + 大号等宽数字 + 辅助说明，加载中显示同形骨架屏。
 */
import SkeletonBlock from './SkeletonBlock.vue';

withDefaults(
  defineProps<{
    /** 指标名称 */
    label: string;
    /** 主数字，渲染为字符串 */
    value: string;
    /** 辅助说明 */
    hint?: string;
    /** 指示条与状态点的语气色 */
    tone?: 'neutral' | 'success' | 'warning' | 'danger';
    /** 加载中显示骨架屏 */
    loading?: boolean;
  }>(),
  { hint: '', tone: 'neutral', loading: false },
);
</script>

<template>
  <article class="stat-card">
    <i class="stat-card__bar" :class="`stat-card__bar--${tone}`" aria-hidden="true" />
    <p class="stat-card__label">{{ label }}</p>
    <p v-if="loading" class="stat-card__skeleton">
      <SkeletonBlock width="4.5rem" height="1.9rem" />
    </p>
    <p v-else class="stat-card__value mcnp-num">{{ value }}</p>
    <p v-if="hint" class="stat-card__hint">{{ hint }}</p>
  </article>
</template>

<style scoped>
.stat-card {
  position: relative;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  padding: 1.1rem 1.25rem 1rem;
  background: var(--mcnp-surface);
  box-shadow: var(--mcnp-shadow);
}

.stat-card__bar {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 3px;
  background: var(--mcnp-gradient-primary);
}

.stat-card__bar--success {
  background: linear-gradient(135deg, var(--mcnp-success), var(--mcnp-accent));
}

.stat-card__bar--warning {
  background: linear-gradient(135deg, var(--mcnp-warning), var(--mcnp-primary));
}

.stat-card__bar--danger {
  background: linear-gradient(135deg, var(--mcnp-danger), var(--mcnp-warning));
}

.stat-card__label {
  margin: 0 0 0.4rem;
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.stat-card__value {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 1.75rem;
  font-weight: 700;
  line-height: 1.2;
}

.stat-card__hint {
  margin: 0.45rem 0 0;
  color: var(--mcnp-text-faint);
  font-size: 0.72rem;
}

.stat-card__skeleton {
  margin: 0;
}
</style>
