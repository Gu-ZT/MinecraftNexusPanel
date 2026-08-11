<script setup lang="ts">
/*
 * StatusBadge：状态徽章（docs/design/frontend-design.md §5）。
 * 状态点 + 柔和底色 + 文字；语气色由 tone 决定，未显式传入时按状态字符串推导。
 */
import { computed } from 'vue';

type BadgeTone = 'neutral' | 'success' | 'warning' | 'danger';

const props = withDefaults(
  defineProps<{
    /** 领域状态值（如 RUNNING、STOPPED、FAILED），用于推导语气色 */
    status: string;
    /** 显示文案，缺省时直接显示 status 原文 */
    label?: string;
    /** 显式指定语气色，覆盖按状态推导的结果 */
    tone?: BadgeTone;
  }>(),
  { label: '' },
);

const effectiveTone = computed<BadgeTone>(() => {
  if (props.tone) {
    return props.tone;
  }
  const normalized = props.status.toLocaleLowerCase();
  if (['running', 'online', 'ready', 'completed', 'success'].includes(normalized)) {
    return 'success';
  }
  if (['starting', 'stopping', 'degraded', 'pending', 'retrying'].includes(normalized)) {
    return 'warning';
  }
  if (
    ['failed', 'error', 'auth-failed', 'incompatible', 'crashed', 'rejected', 'killed'].includes(normalized)
  ) {
    return 'danger';
  }
  return 'neutral';
});
</script>

<template>
  <span class="status-badge" :class="`status-badge--${effectiveTone}`">
    <i class="status-badge__dot" aria-hidden="true" />
    <span class="status-badge__text">{{ label || status }}</span>
  </span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  border-radius: 999px;
  padding: 0.2rem 0.6rem;
  font-size: 0.68rem;
  font-weight: 600;
  line-height: 1.4;
  white-space: nowrap;
}

.status-badge--success {
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
}

.status-badge--warning {
  background: var(--mcnp-warning-soft);
  color: var(--mcnp-warning);
}

.status-badge--danger {
  background: var(--mcnp-danger-soft);
  color: var(--mcnp-danger);
}

.status-badge--neutral {
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
}

.status-badge__dot {
  width: 0.38rem;
  height: 0.38rem;
  border-radius: 50%;
  background: currentColor;
}

.status-badge__text {
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
